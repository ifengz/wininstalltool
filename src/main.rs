mod config;
mod engine;
mod ui_model;

use crate::config::{AppManifest, LoadConfigError};
use crate::engine::{
    DetectionState, DownloadStatus, build_install_command, detect_app, detect_selected_apps,
    download_cache_for_app, run_install_command, validate_manifest_for_install,
};
use crate::ui_model::{CategoryView, InstallerViewModel, build_view_model};
use slint::{ModelRc, SharedString, StandardListViewItem, TableColumn, VecModel};
use std::cell::RefCell;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;
    let state = Rc::new(RefCell::new(RuntimeState::load()));

    refresh_window(&app, &state.borrow());
    wire_callbacks(&app, Rc::clone(&state));

    app.run()
}

struct RuntimeState {
    manifest: Result<AppManifest, LoadConfigError>,
    selected: Vec<String>,
    active_category: Option<String>,
    current_row: i32,
    install_root: String,
    cache_root: String,
    task_status: String,
    task_progress: f32,
    logs: Vec<String>,
}

impl RuntimeState {
    fn load() -> Self {
        let manifest = AppManifest::load_from_default_path();
        let install_root = manifest
            .as_ref()
            .map(|manifest| manifest.default_install_root.clone())
            .unwrap_or_else(|_| "C:\\Program Files\\CompanyApps".to_owned());
        let selected = manifest
            .as_ref()
            .map(|manifest| {
                manifest
                    .apps
                    .iter()
                    .filter(|app| app.enabled_by_default)
                    .map(|app| app.id.clone())
                    .collect()
            })
            .unwrap_or_default();

        Self {
            manifest,
            selected,
            active_category: None,
            current_row: -1,
            install_root,
            cache_root: "cache".to_owned(),
            task_status: "就绪".to_owned(),
            task_progress: 0.0,
            logs: vec!["[startup] 配置已读取，真实安装尚未启用".to_owned()],
        }
    }

    fn push_log(&mut self, message: impl Into<String>) {
        let line = format!("[{}] {}", log_timestamp(), message.into());
        self.logs.push(line.clone());
        if self.logs.len() > 200 {
            self.logs.remove(0);
        }
        append_log_line(&line);
    }
}

fn wire_callbacks(app: &AppWindow, state: Rc<RefCell<RuntimeState>>) {
    let weak = app.as_weak();
    let choose_state = Rc::clone(&state);
    app.on_choose_install_root(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = choose_state.borrow_mut();
        let selected_path = pick_folder("选择默认安装路径", &state.install_root);
        if apply_folder_selection(&mut state.install_root, selected_path) {
            let path = state.install_root.clone();
            state.push_log(format!("默认安装路径已更新：{path}"));
            refresh_window(&app, &state);
        }
    });

    let weak = app.as_weak();
    let cache_state = Rc::clone(&state);
    app.on_choose_cache_root(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = cache_state.borrow_mut();
        sync_editable_paths(&app, &mut state);
        let selected_path = pick_folder("选择下载缓存目录", &state.cache_root);
        if apply_folder_selection(&mut state.cache_root, selected_path) {
            let path = state.cache_root.clone();
            state.push_log(format!("下载缓存目录已更新：{path}"));
            refresh_window(&app, &state);
        }
    });

    let weak = app.as_weak();
    let open_cache_state = Rc::clone(&state);
    app.on_open_cache_root(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = open_cache_state.borrow_mut();
        sync_editable_paths(&app, &mut state);
        open_cache_folder(&mut state);
        refresh_window(&app, &state);
    });

    let weak = app.as_weak();
    let homepage_state = Rc::clone(&state);
    app.on_open_homepage(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = homepage_state.borrow_mut();
        open_homepage_for_current_row(&mut state, app.get_current_row());
        refresh_window(&app, &state);
    });

    let weak = app.as_weak();
    let install_state = Rc::clone(&state);
    app.on_start_install(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = install_state.borrow_mut();
        sync_editable_paths(&app, &mut state);
        run_selected_install(&app, &mut state);
        refresh_window(&app, &state);
    });

    let weak = app.as_weak();
    let download_state = Rc::clone(&state);
    app.on_download_cache(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = download_state.borrow_mut();
        sync_editable_paths(&app, &mut state);
        run_download_cache_plan(&app, &mut state);
        refresh_window(&app, &state);
    });

    let weak = app.as_weak();
    let validate_state = Rc::clone(&state);
    app.on_validate_config(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = validate_state.borrow_mut();
        run_config_validation(&mut state);
        refresh_window(&app, &state);
    });

    let weak = app.as_weak();
    let detect_state = Rc::clone(&state);
    app.on_detect_installed(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = detect_state.borrow_mut();
        run_detection(&mut state);
        refresh_window(&app, &state);
    });

    let weak = app.as_weak();
    app.on_category_selected(move |index| {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = state.borrow_mut();
        let Ok(manifest) = &state.manifest else {
            return;
        };
        state.active_category =
            category_by_index(manifest, index as usize).and_then(|category| category.id);
        state.current_row = -1;
        refresh_window(&app, &state);
    });
}

fn refresh_window(app: &AppWindow, state: &RuntimeState) {
    app.set_install_root(state.install_root.clone().into());
    app.set_cache_root(state.cache_root.clone().into());
    app.set_task_status(state.task_status.clone().into());
    app.set_task_progress(state.task_progress);
    app.set_table_columns(table_columns());
    app.set_log_text(state.logs.join("\n").into());
    app.set_current_row(state.current_row);

    match &state.manifest {
        Ok(manifest) => {
            let view = build_view_model(
                manifest,
                &state.selected,
                state.active_category.as_deref(),
                &state.install_root,
            );
            apply_view_model(app, view);
        }
        Err(error) => {
            app.set_selected_count(0);
            app.set_admin_count(0);
            app.set_verification_count(0);
            app.set_category_labels(shared_string_model(vec!["全部".to_owned()]));
            app.set_table_rows(table_row_model(vec![vec![
                "".to_owned(),
                "配置错误".to_owned(),
                "".to_owned(),
                error.to_string(),
                "".to_owned(),
                "".to_owned(),
                "".to_owned(),
            ]]));
        }
    }
}

fn apply_view_model(app: &AppWindow, view: InstallerViewModel) {
    app.set_selected_count(view.selected_count as i32);
    app.set_admin_count(view.admin_count as i32);
    app.set_verification_count(view.needs_verification_count as i32);
    app.set_category_labels(shared_string_model(
        view.categories
            .iter()
            .map(category_button_label)
            .collect::<Vec<_>>(),
    ));
    app.set_table_rows(table_row_model(
        view.rows
            .into_iter()
            .map(|row| {
                vec![
                    if row.selected { "✓" } else { "" }.to_owned(),
                    row.name,
                    row.purpose,
                    row.source,
                    row.verification,
                    row.install_method,
                    row.path,
                ]
            })
            .collect(),
    ));
}

fn table_columns() -> ModelRc<TableColumn> {
    let columns = [
        ("选", 44.0, 0.0),
        ("软件", 150.0, 1.0),
        ("作用", 86.0, 0.0),
        ("来源", 250.0, 2.0),
        ("验证", 106.0, 0.0),
        ("安装", 106.0, 0.0),
        ("路径", 180.0, 1.0),
    ]
    .into_iter()
    .map(|(title, width, stretch)| {
        let mut column = TableColumn::default();
        column.title = title.into();
        column.min_width = width;
        column.width = width;
        column.horizontal_stretch = stretch;
        column
    })
    .collect::<Vec<_>>();

    Rc::new(VecModel::from(columns)).into()
}

fn table_row_model(rows: Vec<Vec<String>>) -> ModelRc<ModelRc<StandardListViewItem>> {
    let rows = rows
        .into_iter()
        .map(|cells| {
            let cells = cells
                .into_iter()
                .map(|text| {
                    let mut item = StandardListViewItem::default();
                    item.text = text.into();
                    item
                })
                .collect::<Vec<_>>();
            Rc::new(VecModel::from(cells)).into()
        })
        .collect::<Vec<_>>();

    Rc::new(VecModel::from(rows)).into()
}

fn shared_string_model(values: Vec<String>) -> ModelRc<SharedString> {
    let values = values
        .into_iter()
        .map(SharedString::from)
        .collect::<Vec<_>>();
    Rc::new(VecModel::from(values)).into()
}

fn category_button_label(category: &CategoryView) -> String {
    format!("{} {}", category.label, category.count)
}

fn category_by_index(manifest: &AppManifest, index: usize) -> Option<CategoryView> {
    build_view_model(manifest, &[], None, "")
        .categories
        .into_iter()
        .nth(index)
}

fn run_config_validation(state: &mut RuntimeState) {
    match &state.manifest {
        Ok(manifest) => {
            let report = validate_manifest_for_install(manifest);
            if report.errors.is_empty() {
                state.push_log("配置校验通过");
            } else {
                state.push_log(format!("配置校验失败：{} 个错误", report.errors.len()));
                for error in report.errors {
                    state.push_log(format!("错误：{error}"));
                }
            }

            for warning in report.warnings {
                state.push_log(format!("警告：{warning}"));
            }
        }
        Err(error) => state.push_log(format!("配置读取失败：{error}")),
    }
}

fn run_detection(state: &mut RuntimeState) {
    match &state.manifest {
        Ok(manifest) => {
            if state.selected.is_empty() {
                state.push_log("检测跳过：未选择软件");
                return;
            }

            let reports = detect_selected_apps(manifest, &state.selected);
            state.push_log(format!("开始检测已安装状态：{} 个软件", reports.len()));
            for report in reports {
                state.push_log(format!(
                    "{} / {}：{}（{}）",
                    report.app_id,
                    report.app_name,
                    detection_state_label(&report.state),
                    report.details
                ));
            }
        }
        Err(error) => state.push_log(format!("检测失败：配置读取失败：{error}")),
    }
}

fn run_selected_install(app_window: &AppWindow, state: &mut RuntimeState) {
    let manifest = match &state.manifest {
        Ok(manifest) => manifest.clone(),
        Err(error) => {
            state.push_log(format!("安装失败：配置读取失败：{error}"));
            set_task_status(app_window, state, "安装失败：配置读取失败", 0.0);
            return;
        }
    };

    let validation = validate_manifest_for_install(&manifest);
    if !validation.errors.is_empty() {
        state.push_log(format!(
            "安装中止：配置存在 {} 个错误",
            validation.errors.len()
        ));
        for error in validation.errors {
            state.push_log(format!("错误：{error}"));
        }
        set_task_status(app_window, state, "安装中止：配置错误", 0.0);
        return;
    }

    if state.selected.is_empty() {
        state.push_log("安装跳过：未选择软件");
        set_task_status(app_window, state, "安装跳过：未选择软件", 0.0);
        return;
    }

    let selected = state.selected.clone();
    let total = selected.len();
    let cache_root = PathBuf::from(&state.cache_root);
    state.push_log(format!("开始顺序安装：{} 个软件", selected.len()));
    set_task_status(app_window, state, format!("安装准备 0/{total}"), 0.0);
    for (index, app) in manifest
        .apps
        .iter()
        .filter(|app| selected.iter().any(|id| id == &app.id))
        .enumerate()
    {
        set_task_progress(app_window, state, "安装中", index, total, &app.name);

        if app.install.method == "winget_if_missing"
            && detect_app(app).state == DetectionState::Installed
        {
            state.push_log(format!("跳过已安装：{}", app.name));
            set_task_progress(app_window, state, "安装中", index + 1, total, &app.name);
            continue;
        }

        let command = match build_install_command(app, &state.install_root, &cache_root) {
            Ok(command) => command,
            Err(error) => {
                state.push_log(format!("{} 安装失败：{error}", app.name));
                set_task_progress(app_window, state, "安装中", index + 1, total, &app.name);
                continue;
            }
        };

        state.push_log(format!("执行安装：{} / {}", app.name, command.render()));
        let result = run_install_command(&command);
        state.push_log(format!(
            "{} 安装{}，退出码：{}",
            result.app_name,
            if result.success { "成功" } else { "失败" },
            result
                .exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "无".to_owned())
        ));

        if !result.stderr.trim().is_empty() {
            state.push_log(format!(
                "{} stderr：{}",
                result.app_name,
                result.stderr.trim()
            ));
        }
        if !result.stdout.trim().is_empty() {
            state.push_log(format!(
                "{} stdout：{}",
                result.app_name,
                result.stdout.trim()
            ));
        }
        set_task_progress(app_window, state, "安装中", index + 1, total, &app.name);
    }
    set_task_status(
        app_window,
        state,
        format!("安装流程结束 {total}/{total}"),
        1.0,
    );
}

fn run_download_cache_plan(app_window: &AppWindow, state: &mut RuntimeState) {
    let manifest = match &state.manifest {
        Ok(manifest) => manifest.clone(),
        Err(error) => {
            state.push_log(format!("下载缓存失败：配置读取失败：{error}"));
            set_task_status(app_window, state, "下载失败：配置读取失败", 0.0);
            return;
        }
    };

    if state.selected.is_empty() {
        state.push_log("下载缓存跳过：未选择软件");
        set_task_status(app_window, state, "下载跳过：未选择软件", 0.0);
        return;
    }

    let selected = state.selected.clone();
    let total = selected.len();
    let cache_root = PathBuf::from(&state.cache_root);
    state.push_log(format!("检查下载缓存任务：{} 个软件", selected.len()));
    set_task_status(app_window, state, format!("下载准备 0/{total}"), 0.0);
    for (index, app) in manifest
        .apps
        .iter()
        .filter(|app| selected.iter().any(|id| id == &app.id))
        .enumerate()
    {
        set_task_progress(app_window, state, "下载缓存", index, total, &app.name);
        match app.source.source_type.as_str() {
            "winget" | "preinstalled_or_winget" | "direct_url" | "github_release" => {
                let result = download_cache_for_app(app, &cache_root);
                match result.status {
                    DownloadStatus::Downloaded => {
                        let path = result
                            .path
                            .as_ref()
                            .map(|path| path.display().to_string())
                            .unwrap_or_else(|| result.message.clone());
                        state.push_log(format!("{}：下载完成：{path}", result.app_name));
                    }
                    DownloadStatus::Skipped => {
                        state
                            .push_log(format!("{}：跳过下载：{}", result.app_name, result.message));
                    }
                    DownloadStatus::Failed => {
                        state
                            .push_log(format!("{}：下载失败：{}", result.app_name, result.message));
                    }
                }
            }
            source_type => state.push_log(format!(
                "{}：不支持的下载来源类型 `{source_type}`",
                app.name
            )),
        }
        set_task_progress(app_window, state, "下载缓存", index + 1, total, &app.name);
    }
    set_task_status(
        app_window,
        state,
        format!("下载缓存结束 {total}/{total}"),
        1.0,
    );
}

fn detection_state_label(state: &DetectionState) -> &'static str {
    match state {
        DetectionState::Installed => "已安装",
        DetectionState::NotInstalled => "未安装",
        DetectionState::Unsupported => "无法检测",
        DetectionState::Error => "检测错误",
    }
}

fn open_homepage_for_current_row(state: &mut RuntimeState, current_row: i32) {
    let Ok(manifest) = &state.manifest else {
        state.push_log("打开官网失败：配置未正确读取");
        return;
    };

    let Some(row) = visible_row_by_index(
        manifest,
        &state.selected,
        state.active_category.as_deref(),
        &state.install_root,
        current_row,
    ) else {
        state.push_log("打开官网失败：未选择有效软件行");
        return;
    };

    if row.homepage_url.trim().is_empty() {
        state.push_log(format!("打开官网失败：{} 未配置官网地址", row.name));
        return;
    }

    match webbrowser::open(&row.homepage_url) {
        Ok(_) => state.push_log(format!("已打开官网：{}", row.name)),
        Err(error) => state.push_log(format!("打开官网失败：{}：{error}", row.name)),
    }
}

fn visible_row_by_index(
    manifest: &AppManifest,
    selected: &[String],
    active_category: Option<&str>,
    install_root: &str,
    index: i32,
) -> Option<ui_model::AppRowView> {
    let index = usize::try_from(index).ok()?;
    build_view_model(manifest, selected, active_category, install_root)
        .rows
        .into_iter()
        .nth(index)
}

fn pick_folder(title: &str, current_root: &str) -> Option<PathBuf> {
    let dialog = rfd::FileDialog::new().set_title(title);
    let current_path = Path::new(current_root);
    let dialog = if current_path.exists() {
        dialog.set_directory(current_path)
    } else {
        dialog
    };

    dialog.pick_folder()
}

fn apply_folder_selection(target: &mut String, selected_path: Option<PathBuf>) -> bool {
    let Some(path) = selected_path else {
        return false;
    };

    let path_text = path.to_string_lossy().trim().to_owned();
    if path_text.is_empty() || *target == path_text {
        return false;
    }

    *target = path_text;
    true
}

fn sync_editable_paths(app: &AppWindow, state: &mut RuntimeState) {
    let install_root = app.get_install_root().trim().to_owned();
    if !install_root.is_empty() {
        state.install_root = install_root;
    }

    let cache_root = app.get_cache_root().trim().to_owned();
    if !cache_root.is_empty() {
        state.cache_root = cache_root;
    }
}

fn open_cache_folder(state: &mut RuntimeState) {
    let path = PathBuf::from(&state.cache_root);
    match std::fs::create_dir_all(&path).and_then(|_| open_path_in_file_manager(&path)) {
        Ok(()) => state.push_log(format!("已打开下载缓存目录：{}", path.display())),
        Err(error) => state.push_log(format!("打开下载缓存目录失败：{}：{error}", path.display())),
    }
}

fn open_path_in_file_manager(path: &Path) -> std::io::Result<()> {
    let status = file_manager_command(path).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "文件管理器退出码：{}",
            status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "无".to_owned())
        )))
    }
}

fn file_manager_command(path: &Path) -> Command {
    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("explorer");
        command.arg(path);
        command
    }

    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new("open");
        command.arg(path);
        command
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        let mut command = Command::new("xdg-open");
        command.arg(path);
        command
    }
}

fn set_task_progress(
    app: &AppWindow,
    state: &mut RuntimeState,
    phase: &str,
    completed: usize,
    total: usize,
    item: &str,
) {
    let progress = if total == 0 {
        0.0
    } else {
        completed as f32 / total as f32
    };
    set_task_status(
        app,
        state,
        format!("{phase} {completed}/{total}：{item}"),
        progress,
    );
}

fn set_task_status(
    app: &AppWindow,
    state: &mut RuntimeState,
    status: impl Into<String>,
    progress: f32,
) {
    state.task_status = status.into();
    state.task_progress = progress.clamp(0.0, 1.0);
    app.set_task_status(state.task_status.clone().into());
    app.set_task_progress(state.task_progress);
}

fn log_timestamp() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    seconds.to_string()
}

fn append_log_line(line: &str) {
    let _ = std::fs::create_dir_all("logs");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("logs/install.log")
    {
        let _ = writeln!(file, "{line}");
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    #[test]
    fn install_root_updates_when_folder_is_selected() {
        let mut install_root = "C:\\Program Files\\CompanyApps".to_owned();

        let changed = super::apply_folder_selection(
            &mut install_root,
            Some(PathBuf::from("D:\\CompanyApps")),
        );

        assert!(changed);
        assert_eq!(install_root, "D:\\CompanyApps");
    }

    #[test]
    fn install_root_is_unchanged_when_picker_is_cancelled() {
        let mut install_root = "C:\\Program Files\\CompanyApps".to_owned();

        let changed = super::apply_folder_selection(&mut install_root, None);

        assert!(!changed);
        assert_eq!(install_root, "C:\\Program Files\\CompanyApps");
    }

    #[test]
    fn visible_row_lookup_uses_active_category_order() {
        let manifest =
            crate::config::AppManifest::load_from_default_path().expect("apps example should load");
        let selected = manifest
            .apps
            .iter()
            .map(|app| app.id.clone())
            .collect::<Vec<_>>();

        let row = super::visible_row_by_index(&manifest, &selected, Some("browser"), "D:\\Apps", 1)
            .expect("browser row should exist");

        assert_eq!(row.name, "Microsoft Edge");
        assert!(row.homepage_url.starts_with("https://"));
    }

    #[test]
    fn cache_root_updates_when_folder_is_selected() {
        let mut cache_root = "cache".to_owned();

        let changed = super::apply_folder_selection(
            &mut cache_root,
            Some(PathBuf::from("E:\\InstallerCache")),
        );

        assert!(changed);
        assert_eq!(cache_root, "E:\\InstallerCache");
    }
}
