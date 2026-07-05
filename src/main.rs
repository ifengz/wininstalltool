#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod config;
mod engine;
mod ui_model;

use crate::config::{
    AppEntry, AppManifest, DEFAULT_CONFIG_PATH, DetectRule, DetectSpec, InstallSpec,
    LoadConfigError, PackageSource,
};
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

const INSTALL_LOG_PATH: &str = "logs/install.log";
const TABLE_COLUMNS: [(&str, f32, f32); 7] = [
    ("", 38.0, 0.0),
    ("软件", 132.0, 1.0),
    ("作用", 74.0, 0.0),
    ("来源", 220.0, 2.0),
    ("验证", 100.0, 0.0),
    ("安装", 86.0, 0.0),
    ("路径", 150.0, 1.0),
];
const DEFAULT_NEW_APP_CATEGORY: &str = "通用软件";

fn main() -> Result<(), slint::PlatformError> {
    configure_windows_renderer()?;
    let app = AppWindow::new()?;
    let state = Rc::new(RefCell::new(RuntimeState::load()));

    refresh_window(&app, &state.borrow());
    wire_callbacks(&app, Rc::clone(&state));

    app.run()
}

#[cfg(target_os = "windows")]
fn configure_windows_renderer() -> Result<(), slint::PlatformError> {
    slint::BackendSelector::new()
        .backend_name("winit".to_owned())
        .renderer_name("software".to_owned())
        .select()
}

#[cfg(not(target_os = "windows"))]
fn configure_windows_renderer() -> Result<(), slint::PlatformError> {
    Ok(())
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
    pending_delete_app_id: Option<String>,
    pending_delete_app_name: Option<String>,
    add_form: AddAppFormState,
    logs: Vec<String>,
}

#[derive(Clone, Debug, Default)]
struct AddAppFormState {
    visible: bool,
    editing_existing_id: Option<String>,
    id: String,
    name: String,
    category: String,
    winget_id: String,
    homepage: String,
    detect_path: String,
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
            pending_delete_app_id: None,
            pending_delete_app_name: None,
            add_form: AddAppFormState::default(),
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
    let row_homepage_state = Rc::clone(&state);
    app.on_open_row_homepage(move |row| {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = row_homepage_state.borrow_mut();
        state.current_row = row;
        open_homepage_for_current_row(&mut state, row);
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
    let add_template_state = Rc::clone(&state);
    app.on_show_add_app_form(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = add_template_state.borrow_mut();
        show_add_app_form(&mut state);
        refresh_window(&app, &state);
    });

    let weak = app.as_weak();
    let edit_app_state = Rc::clone(&state);
    app.on_show_edit_current_app_form(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = edit_app_state.borrow_mut();
        show_edit_current_app_form(&mut state, app.get_current_row());
        refresh_window(&app, &state);
    });

    let weak = app.as_weak();
    let edit_row_state = Rc::clone(&state);
    app.on_show_edit_row_form(move |row| {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = edit_row_state.borrow_mut();
        state.current_row = row;
        show_edit_current_app_form(&mut state, row);
        refresh_window(&app, &state);
    });

    let weak = app.as_weak();
    let save_add_state = Rc::clone(&state);
    app.on_save_add_app_form(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = save_add_state.borrow_mut();
        sync_add_form(&app, &mut state);
        save_add_app_form(&mut state);
        refresh_window(&app, &state);
    });

    let weak = app.as_weak();
    let cancel_add_state = Rc::clone(&state);
    app.on_cancel_add_app_form(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = cancel_add_state.borrow_mut();
        state.add_form.visible = false;
        state.push_log("已取消添加软件");
        refresh_window(&app, &state);
    });

    let weak = app.as_weak();
    let open_config_state = Rc::clone(&state);
    app.on_open_config(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = open_config_state.borrow_mut();
        open_config_file(&mut state);
        refresh_window(&app, &state);
    });

    let weak = app.as_weak();
    let reload_config_state = Rc::clone(&state);
    app.on_reload_config(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = reload_config_state.borrow_mut();
        reload_config(&mut state);
        refresh_window(&app, &state);
    });

    let weak = app.as_weak();
    let delete_app_state = Rc::clone(&state);
    app.on_request_delete_current_app(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = delete_app_state.borrow_mut();
        request_delete_current_app(&mut state, app.get_current_row());
        refresh_window(&app, &state);
    });

    let weak = app.as_weak();
    let delete_row_state = Rc::clone(&state);
    app.on_request_delete_row(move |row| {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = delete_row_state.borrow_mut();
        state.current_row = row;
        request_delete_current_app(&mut state, row);
        refresh_window(&app, &state);
    });

    let weak = app.as_weak();
    let confirm_delete_state = Rc::clone(&state);
    app.on_confirm_delete_current_app(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = confirm_delete_state.borrow_mut();
        confirm_delete_current_app(&mut state);
        refresh_window(&app, &state);
    });

    let weak = app.as_weak();
    let cancel_delete_state = Rc::clone(&state);
    app.on_cancel_delete_current_app(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = cancel_delete_state.borrow_mut();
        clear_pending_delete(&mut state);
        state.push_log("已取消删除软件");
        refresh_window(&app, &state);
    });

    let weak = app.as_weak();
    let open_log_state = Rc::clone(&state);
    app.on_open_log(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = open_log_state.borrow_mut();
        open_log_file(&mut state);
        refresh_window(&app, &state);
    });

    let weak = app.as_weak();
    let clear_log_state = Rc::clone(&state);
    app.on_clear_log(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = clear_log_state.borrow_mut();
        clear_logs(&mut state);
        refresh_window(&app, &state);
    });

    let weak = app.as_weak();
    let export_log_state = Rc::clone(&state);
    app.on_export_log(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = export_log_state.borrow_mut();
        export_logs(&mut state);
        refresh_window(&app, &state);
    });

    let weak = app.as_weak();
    let category_state = Rc::clone(&state);
    app.on_category_selected(move |index| {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = category_state.borrow_mut();
        let Ok(manifest) = &state.manifest else {
            return;
        };
        state.active_category =
            category_by_index(manifest, index as usize).and_then(|category| category.id);
        state.current_row = -1;
        refresh_window(&app, &state);
    });

    let weak = app.as_weak();
    let toggle_state = Rc::clone(&state);
    app.on_toggle_row_selection(move |row| {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = toggle_state.borrow_mut();
        state.current_row = row;
        toggle_selected_app_by_visible_row(&mut state, row);
        refresh_window(&app, &state);
    });

    let weak = app.as_weak();
    let toggle_visible_state = Rc::clone(&state);
    app.on_toggle_visible_selection(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };

        let mut state = toggle_visible_state.borrow_mut();
        toggle_visible_selection(&mut state);
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
    app.set_delete_confirm_visible(state.pending_delete_app_id.is_some());
    app.set_delete_confirm_text(
        state
            .pending_delete_app_name
            .as_ref()
            .map(|name| format!("确认从配置删除：{name}？此操作会写回软件清单。"))
            .unwrap_or_default()
            .into(),
    );
    app.set_add_form_visible(state.add_form.visible);
    app.set_add_app_id(state.add_form.id.clone().into());
    app.set_add_app_name(state.add_form.name.clone().into());
    app.set_add_app_category(state.add_form.category.clone().into());
    app.set_add_app_winget_id(state.add_form.winget_id.clone().into());
    app.set_add_app_homepage(state.add_form.homepage.clone().into());
    app.set_add_app_detect_path(state.add_form.detect_path.clone().into());
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
            app.set_all_visible_selected(false);
            app.set_category_labels(shared_string_model(vec!["全部".to_owned()]));
            app.set_app_rows(app_row_model(Vec::new()));
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
    app.set_app_rows(app_row_model(view.rows.clone()));
    app.set_all_visible_selected(!view.rows.is_empty() && view.rows.iter().all(|row| row.selected));
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
    let columns = TABLE_COLUMNS
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

fn app_row_model(rows: Vec<crate::ui_model::AppRowView>) -> ModelRc<UiAppRow> {
    let rows = rows
        .into_iter()
        .map(|row| UiAppRow {
            selected: row.selected,
            name: row.name.into(),
            purpose: row.purpose.into(),
            source: row.source.into(),
            verification: row.verification.into(),
            install_method: row.install_method.into(),
            path: row.path.into(),
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

fn open_config_file(state: &mut RuntimeState) {
    let path = Path::new(DEFAULT_CONFIG_PATH);
    match open_path_in_file_manager(path) {
        Ok(()) => state.push_log(format!("已打开软件配置：{DEFAULT_CONFIG_PATH}")),
        Err(error) => state.push_log(format!("打开软件配置失败：{DEFAULT_CONFIG_PATH}：{error}")),
    }
}

fn reload_config(state: &mut RuntimeState) {
    match AppManifest::load_from_default_path() {
        Ok(manifest) => {
            state.selected = manifest
                .apps
                .iter()
                .filter(|app| app.enabled_by_default)
                .map(|app| app.id.clone())
                .collect();
            state.active_category = None;
            state.current_row = -1;
            state.install_root = manifest.default_install_root.clone();
            state.manifest = Ok(manifest);
            state.push_log("软件配置已重载");
        }
        Err(error) => {
            state.manifest = Err(error);
            state.selected.clear();
            state.current_row = -1;
            state.push_log("软件配置重载失败");
        }
    }
}

fn show_add_app_form(state: &mut RuntimeState) {
    let manifest = match &state.manifest {
        Ok(manifest) => manifest,
        Err(_) => {
            state.push_log("添加软件失败：配置未正确读取");
            return;
        }
    };

    let next_number = next_template_number(manifest);
    state.add_form = AddAppFormState {
        visible: true,
        editing_existing_id: None,
        id: format!("new-app-{next_number}"),
        name: String::new(),
        category: DEFAULT_NEW_APP_CATEGORY.to_owned(),
        winget_id: String::new(),
        homepage: String::new(),
        detect_path: String::new(),
    };
    state.push_log("请填写软件名称、winget 包 ID 和检测路径后保存");
}

fn show_edit_current_app_form(state: &mut RuntimeState, current_row: i32) -> bool {
    let manifest = match &state.manifest {
        Ok(manifest) => manifest,
        Err(_) => {
            state.push_log("编辑软件失败：配置未正确读取");
            return false;
        }
    };

    let Some(app) = visible_app_by_index(manifest, state.active_category.as_deref(), current_row)
    else {
        state.push_log("编辑软件失败：未选择有效软件行");
        return false;
    };

    state.add_form = form_from_app(app);
    state.push_log(format!("正在编辑软件：{}", app.name));
    true
}

fn save_add_app_form(state: &mut RuntimeState) -> bool {
    let manifest = match &state.manifest {
        Ok(manifest) => manifest,
        Err(_) => {
            state.push_log("添加软件失败：配置未正确读取");
            return false;
        }
    };

    let app = match app_from_add_form(&state.add_form) {
        Ok(app) => app,
        Err(error) => {
            state.push_log(format!("添加软件失败：{error}"));
            return false;
        }
    };
    let app_name = app.name.clone();

    let next_manifest =
        match manifest_with_saved_app(manifest, state.add_form.editing_existing_id.as_deref(), app)
        {
            Ok(manifest) => manifest,
            Err(error) => {
                state.push_log(format!("保存软件失败：{error}"));
                return false;
            }
        };

    match next_manifest.save_to_default_path() {
        Ok(()) => {
            state.manifest = Ok(next_manifest);
            state.active_category = None;
            state.current_row = -1;
            state.add_form.visible = false;
            state.push_log(format!(
                "已保存软件：{app_name}。默认不勾选，请校验配置和检测规则后再安装。"
            ));
            true
        }
        Err(error) => {
            state.push_log(format!("添加软件失败：写入配置失败：{error}"));
            false
        }
    }
}

fn form_from_app(app: &AppEntry) -> AddAppFormState {
    AddAppFormState {
        visible: true,
        editing_existing_id: Some(app.id.clone()),
        id: app.id.clone(),
        name: app.name.clone(),
        category: app.category.clone(),
        winget_id: app.source.package_id.clone().unwrap_or_default(),
        homepage: app.homepage_url.clone().unwrap_or_default(),
        detect_path: app
            .detect
            .rules
            .iter()
            .find(|rule| rule.rule_type == "path_exists")
            .map(|rule| rule.value.clone())
            .unwrap_or_default(),
    }
}

fn app_from_add_form(form: &AddAppFormState) -> Result<AppEntry, String> {
    let id = form.id.trim();
    let name = form.name.trim();
    let winget_id = form.winget_id.trim();
    let detect_path = form.detect_path.trim();

    if id.is_empty() {
        return Err("id 不能为空".to_owned());
    }
    if name.is_empty() {
        return Err("名称不能为空".to_owned());
    }
    if winget_id.is_empty() {
        return Err("winget 包 ID 不能为空".to_owned());
    }
    if detect_path.is_empty() {
        return Err("检测路径不能为空".to_owned());
    }

    Ok(AppEntry {
        id: id.to_owned(),
        name: name.to_owned(),
        category: non_empty_or(&form.category, DEFAULT_NEW_APP_CATEGORY),
        homepage_url: optional_trimmed(&form.homepage),
        enabled_by_default: false,
        verification_status: "candidate_medium".to_owned(),
        source: PackageSource {
            source_type: "winget".to_owned(),
            package_id: Some(winget_id.to_owned()),
            url: None,
            repo: None,
            asset_pattern: None,
        },
        install: InstallSpec {
            method: "winget".to_owned(),
            requires_admin: true,
            supports_custom_path: false,
            args: Some(winget_install_args(winget_id)),
            silent_args: None,
            direct_silent_args: None,
            direct_install_location_arg: None,
            fallback_notes: Some("GUI 添加：请实机验证 winget 安装和检测路径".to_owned()),
        },
        detect: DetectSpec {
            detect_type: "path_exists".to_owned(),
            rules: vec![DetectRule {
                rule_type: "path_exists".to_owned(),
                value: detect_path.to_owned(),
            }],
        },
        notes: Some("GUI 添加：默认不勾选，验证后再启用".to_owned()),
    })
}

fn manifest_with_saved_app(
    manifest: &AppManifest,
    editing_existing_id: Option<&str>,
    app: AppEntry,
) -> Result<AppManifest, String> {
    let mut next_manifest = manifest.clone();
    match editing_existing_id {
        Some(existing_id) => {
            let Some(position) = next_manifest
                .apps
                .iter()
                .position(|existing| existing.id == existing_id)
            else {
                return Err(format!("软件不存在：{existing_id}"));
            };
            if app.id != existing_id
                && next_manifest
                    .apps
                    .iter()
                    .any(|existing| existing.id == app.id)
            {
                return Err(format!("软件 id 已存在：{}", app.id));
            }
            next_manifest.apps[position] = app;
        }
        None => {
            if next_manifest
                .apps
                .iter()
                .any(|existing| existing.id == app.id)
            {
                return Err(format!("软件 id 已存在：{}", app.id));
            }
            next_manifest.apps.push(app);
        }
    }
    Ok(next_manifest)
}

fn winget_install_args(package_id: &str) -> Vec<String> {
    [
        "install",
        "--id",
        package_id,
        "--exact",
        "--accept-package-agreements",
        "--accept-source-agreements",
        "--silent",
        "--disable-interactivity",
        "--no-upgrade",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn non_empty_or(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn optional_trimmed(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

fn next_template_number(manifest: &AppManifest) -> usize {
    (1..)
        .find(|number| {
            let id = format!("new-app-{number}");
            manifest.apps.iter().all(|app| app.id != id)
        })
        .unwrap_or(1)
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

fn open_log_file(state: &mut RuntimeState) {
    ensure_log_file_exists();
    match open_path_in_file_manager(Path::new(INSTALL_LOG_PATH)) {
        Ok(()) => state.push_log(format!("已打开日志文件：{INSTALL_LOG_PATH}")),
        Err(error) => state.push_log(format!("打开日志文件失败：{INSTALL_LOG_PATH}：{error}")),
    }
}

fn clear_logs(state: &mut RuntimeState) {
    state.logs.clear();
    if let Err(error) = reset_log_file() {
        state
            .logs
            .push(format!("[{}] 清空日志失败：{}", log_timestamp(), error));
        return;
    }
    state.push_log("日志已清空");
}

fn export_logs(state: &mut RuntimeState) {
    let export_path = PathBuf::from("logs").join(format!(
        "install-{}-{}.log",
        sanitized_machine_name(),
        log_timestamp()
    ));

    match std::fs::create_dir_all("logs")
        .and_then(|_| std::fs::write(&export_path, format!("{}\n", state.logs.join("\n"))))
    {
        Ok(()) => state.push_log(format!("已导出日志：{}", export_path.display())),
        Err(error) => state.push_log(format!("导出日志失败：{error}")),
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
        let report = detect_app(app);
        state.push_log(format!(
            "{} 安装后验证：{}（{}）",
            report.app_name,
            detection_state_label(&report.state),
            report.details
        ));
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

fn request_delete_current_app(state: &mut RuntimeState, current_row: i32) -> bool {
    let manifest = match &state.manifest {
        Ok(manifest) => manifest,
        Err(_) => {
            state.push_log("请求删除失败：配置未正确读取");
            return false;
        }
    };

    let Some(app) = visible_app_by_index(manifest, state.active_category.as_deref(), current_row)
    else {
        state.push_log("请求删除失败：未选择有效软件行");
        return false;
    };

    state.pending_delete_app_id = Some(app.id.clone());
    state.pending_delete_app_name = Some(app.name.clone());
    state.push_log(format!("等待确认删除软件：{}", app.name));
    true
}

fn confirm_delete_current_app(state: &mut RuntimeState) -> bool {
    let Some(app_id) = state.pending_delete_app_id.clone() else {
        state.push_log("删除软件失败：没有待确认的软件");
        return false;
    };

    let manifest = match &state.manifest {
        Ok(manifest) => manifest,
        Err(_) => {
            clear_pending_delete(state);
            state.push_log("删除软件失败：配置未正确读取");
            return false;
        }
    };

    let Some((next_manifest, removed)) = manifest_without_app_id(manifest, &app_id) else {
        clear_pending_delete(state);
        state.push_log("删除软件失败：软件不存在");
        return false;
    };

    match next_manifest.save_to_default_path() {
        Ok(()) => {
            state.selected.retain(|id| id != &removed.id);
            state.current_row = -1;
            state.manifest = Ok(next_manifest);
            clear_pending_delete(state);
            state.push_log(format!("已从配置删除软件：{}", removed.name));
            true
        }
        Err(error) => {
            clear_pending_delete(state);
            state.push_log(format!("删除软件失败：写入配置失败：{error}"));
            false
        }
    }
}

fn clear_pending_delete(state: &mut RuntimeState) {
    state.pending_delete_app_id = None;
    state.pending_delete_app_name = None;
}

#[cfg(test)]
fn manifest_without_visible_app(
    manifest: &AppManifest,
    active_category: Option<&str>,
    current_row: i32,
) -> Option<(AppManifest, crate::config::AppEntry)> {
    let app_id = visible_app_id_by_index(manifest, active_category, current_row)?;
    manifest_without_app_id(manifest, &app_id)
}

fn manifest_without_app_id(
    manifest: &AppManifest,
    app_id: &str,
) -> Option<(AppManifest, crate::config::AppEntry)> {
    let mut next_manifest = manifest.clone();
    let position = next_manifest.apps.iter().position(|app| app.id == app_id)?;
    let removed = next_manifest.apps.remove(position);
    Some((next_manifest, removed))
}

fn toggle_selected_app_by_visible_row(state: &mut RuntimeState, current_row: i32) -> bool {
    let Ok(manifest) = &state.manifest else {
        state.push_log("切换选择失败：配置未正确读取");
        return false;
    };

    let Some(app_id) =
        visible_app_id_by_index(manifest, state.active_category.as_deref(), current_row)
    else {
        state.push_log("切换选择失败：未选择有效软件行");
        return false;
    };

    if let Some(index) = state.selected.iter().position(|id| id == &app_id) {
        state.selected.remove(index);
        false
    } else {
        state.selected.push(app_id);
        true
    }
}

fn toggle_visible_selection(state: &mut RuntimeState) -> bool {
    let Ok(manifest) = &state.manifest else {
        state.push_log("全选失败：配置未正确读取");
        return false;
    };

    let visible_ids = visible_app_ids(manifest, state.active_category.as_deref());
    if visible_ids.is_empty() {
        state.push_log("全选跳过：当前列表没有软件");
        return false;
    }

    let all_selected = visible_ids
        .iter()
        .all(|id| state.selected.iter().any(|selected| selected == id));
    if all_selected {
        state
            .selected
            .retain(|id| !visible_ids.iter().any(|visible_id| visible_id == id));
        state.push_log(format!("已取消选择当前列表：{} 个软件", visible_ids.len()));
        false
    } else {
        for id in visible_ids.iter() {
            if !state.selected.iter().any(|selected| selected == id) {
                state.selected.push(id.clone());
            }
        }
        state.push_log(format!("已选择当前列表：{} 个软件", visible_ids.len()));
        true
    }
}

fn visible_app_ids(manifest: &AppManifest, active_category: Option<&str>) -> Vec<String> {
    manifest
        .apps
        .iter()
        .filter(|app| active_category.is_none_or(|category| app.category == category))
        .map(|app| app.id.clone())
        .collect()
}

fn visible_app_id_by_index(
    manifest: &AppManifest,
    active_category: Option<&str>,
    index: i32,
) -> Option<String> {
    visible_app_by_index(manifest, active_category, index).map(|app| app.id.clone())
}

fn visible_app_by_index<'a>(
    manifest: &'a AppManifest,
    active_category: Option<&str>,
    index: i32,
) -> Option<&'a AppEntry> {
    let index = usize::try_from(index).ok()?;
    manifest
        .apps
        .iter()
        .filter(|app| active_category.is_none_or(|category| app.category == category))
        .nth(index)
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

fn sync_add_form(app: &AppWindow, state: &mut RuntimeState) {
    state.add_form.id = app.get_add_app_id().trim().to_owned();
    state.add_form.name = app.get_add_app_name().trim().to_owned();
    state.add_form.category = app.get_add_app_category().trim().to_owned();
    state.add_form.winget_id = app.get_add_app_winget_id().trim().to_owned();
    state.add_form.homepage = app.get_add_app_homepage().trim().to_owned();
    state.add_form.detect_path = app.get_add_app_detect_path().trim().to_owned();
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
        .open(INSTALL_LOG_PATH)
    {
        let _ = writeln!(file, "{line}");
    }
}

fn ensure_log_file_exists() {
    let _ = std::fs::create_dir_all("logs");
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(INSTALL_LOG_PATH);
}

fn reset_log_file() -> std::io::Result<()> {
    std::fs::create_dir_all("logs")?;
    std::fs::write(INSTALL_LOG_PATH, "")
}

fn sanitized_machine_name() -> String {
    let name = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown-machine".to_owned());
    let sanitized = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();

    if sanitized.trim_matches('-').is_empty() {
        "unknown-machine".to_owned()
    } else {
        sanitized
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
    fn visible_app_id_lookup_uses_active_category_order() {
        let manifest =
            crate::config::AppManifest::load_from_default_path().expect("apps example should load");

        let app_id = super::visible_app_id_by_index(&manifest, Some("browser"), 1)
            .expect("browser row should exist");

        assert_eq!(app_id, "edge");
    }

    #[test]
    fn visible_row_toggle_updates_selected_ids() {
        let manifest =
            crate::config::AppManifest::load_from_default_path().expect("apps example should load");
        let mut state = super::RuntimeState {
            manifest: Ok(manifest),
            selected: vec!["chrome".to_owned()],
            active_category: Some("browser".to_owned()),
            current_row: 0,
            install_root: "D:\\Apps".to_owned(),
            cache_root: "cache".to_owned(),
            task_status: "就绪".to_owned(),
            task_progress: 0.0,
            pending_delete_app_id: None,
            pending_delete_app_name: None,
            add_form: super::AddAppFormState::default(),
            logs: Vec::new(),
        };

        let selected = super::toggle_selected_app_by_visible_row(&mut state, 0);
        assert!(!selected);
        assert!(!state.selected.iter().any(|id| id == "chrome"));

        let selected = super::toggle_selected_app_by_visible_row(&mut state, 0);
        assert!(selected);
        assert!(state.selected.iter().any(|id| id == "chrome"));
    }

    #[test]
    fn visible_header_toggle_selects_and_clears_active_category() {
        let manifest =
            crate::config::AppManifest::load_from_default_path().expect("apps example should load");
        let mut state = super::RuntimeState {
            manifest: Ok(manifest),
            selected: Vec::new(),
            active_category: Some("browser".to_owned()),
            current_row: -1,
            install_root: "D:\\Apps".to_owned(),
            cache_root: "cache".to_owned(),
            task_status: "就绪".to_owned(),
            task_progress: 0.0,
            pending_delete_app_id: None,
            pending_delete_app_name: None,
            add_form: super::AddAppFormState::default(),
            logs: Vec::new(),
        };

        let selected_all = super::toggle_visible_selection(&mut state);
        assert!(selected_all);
        assert!(state.selected.iter().any(|id| id == "chrome"));
        assert!(state.selected.iter().any(|id| id == "edge"));
        assert!(state.selected.iter().any(|id| id == "vivaldi"));
        assert!(!state.selected.iter().any(|id| id == "feishu"));

        let cleared = super::toggle_visible_selection(&mut state);
        assert!(!cleared);
        assert!(!state.selected.iter().any(|id| id == "chrome"));
        assert!(!state.selected.iter().any(|id| id == "edge"));
        assert!(!state.selected.iter().any(|id| id == "vivaldi"));
    }

    #[test]
    fn manifest_remove_uses_visible_category_row() {
        let manifest =
            crate::config::AppManifest::load_from_default_path().expect("apps example should load");

        let (next_manifest, removed) =
            super::manifest_without_visible_app(&manifest, Some("browser"), 1)
                .expect("browser row should exist");

        assert_eq!(removed.id, "edge");
        assert_eq!(next_manifest.apps.len(), manifest.apps.len() - 1);
        assert!(!next_manifest.apps.iter().any(|app| app.id == "edge"));
        assert!(next_manifest.apps.iter().any(|app| app.id == "chrome"));
    }

    #[test]
    fn delete_request_sets_pending_app_without_removing_it() {
        let manifest =
            crate::config::AppManifest::load_from_default_path().expect("apps example should load");
        let original_len = manifest.apps.len();
        let mut state = super::RuntimeState {
            manifest: Ok(manifest),
            selected: vec!["edge".to_owned()],
            active_category: Some("browser".to_owned()),
            current_row: 1,
            install_root: "D:\\Apps".to_owned(),
            cache_root: "cache".to_owned(),
            task_status: "就绪".to_owned(),
            task_progress: 0.0,
            pending_delete_app_id: None,
            pending_delete_app_name: None,
            add_form: super::AddAppFormState::default(),
            logs: Vec::new(),
        };

        let requested = super::request_delete_current_app(&mut state, 1);

        assert!(requested);
        assert_eq!(state.pending_delete_app_id.as_deref(), Some("edge"));
        assert_eq!(state.manifest.as_ref().unwrap().apps.len(), original_len);
    }

    #[test]
    fn manifest_remove_by_id_removes_exact_app() {
        let manifest =
            crate::config::AppManifest::load_from_default_path().expect("apps example should load");

        let (next_manifest, removed) =
            super::manifest_without_app_id(&manifest, "edge").expect("edge should exist");

        assert_eq!(removed.id, "edge");
        assert_eq!(next_manifest.apps.len(), manifest.apps.len() - 1);
        assert!(!next_manifest.apps.iter().any(|app| app.id == "edge"));
    }

    #[test]
    fn show_add_form_uses_next_unique_id() {
        let manifest =
            crate::config::AppManifest::load_from_default_path().expect("apps example should load");
        let mut state = super::RuntimeState {
            manifest: Ok(manifest),
            selected: Vec::new(),
            active_category: None,
            current_row: -1,
            install_root: "D:\\Apps".to_owned(),
            cache_root: "cache".to_owned(),
            task_status: "就绪".to_owned(),
            task_progress: 0.0,
            pending_delete_app_id: None,
            pending_delete_app_name: None,
            add_form: super::AddAppFormState::default(),
            logs: Vec::new(),
        };

        super::show_add_app_form(&mut state);

        assert!(state.add_form.visible);
        assert!(state.add_form.id.starts_with("new-app-"));
        assert!(
            state
                .manifest
                .as_ref()
                .unwrap()
                .apps
                .iter()
                .all(|app| app.id != state.add_form.id)
        );
        assert_eq!(state.add_form.category, super::DEFAULT_NEW_APP_CATEGORY);
    }

    #[test]
    fn table_columns_fit_default_right_panel_with_inset() {
        let guarded_width = 820.0;
        let total_width = super::TABLE_COLUMNS
            .iter()
            .map(|(_, width, _)| width)
            .sum::<f32>();

        assert!(
            total_width <= guarded_width,
            "table minimum width {total_width} exceeds guarded right-panel width {}",
            guarded_width
        );
    }

    #[test]
    fn e2e_add_real_winget_app_save_reload_and_delete() {
        let manifest =
            crate::config::AppManifest::load_from_default_path().expect("apps example should load");
        let form = super::AddAppFormState {
            visible: true,
            editing_existing_id: None,
            id: "sevenzip-e2e".to_owned(),
            name: "7-Zip E2E".to_owned(),
            category: "utility".to_owned(),
            winget_id: "7zip.7zip".to_owned(),
            homepage: "https://www.7-zip.org/".to_owned(),
            detect_path: "C:\\Program Files\\7-Zip\\7zFM.exe".to_owned(),
        };

        let app = super::app_from_add_form(&form).expect("real winget app form should build");
        let added_manifest =
            super::manifest_with_saved_app(&manifest, None, app).expect("app should be added");
        let report = crate::engine::validate_manifest_for_install(&added_manifest);
        assert!(report.errors.is_empty(), "{:?}", report.errors);

        let path = std::env::temp_dir().join(format!(
            "wininstalltool-e2e-{}.json",
            super::log_timestamp()
        ));
        added_manifest
            .save_to_path(&path)
            .expect("added manifest should save");
        let reloaded =
            crate::config::AppManifest::load_from_path(&path).expect("manifest should reload");
        assert!(reloaded.apps.iter().any(|app| app.id == "sevenzip-e2e"));
        assert!(
            !reloaded
                .apps
                .iter()
                .find(|app| app.id == "sevenzip-e2e")
                .unwrap()
                .enabled_by_default
        );

        let (deleted_manifest, removed) = super::manifest_without_app_id(&reloaded, "sevenzip-e2e")
            .expect("added app should be removable");
        assert_eq!(removed.source.package_id.as_deref(), Some("7zip.7zip"));
        deleted_manifest
            .save_to_path(&path)
            .expect("deleted manifest should save");
        let final_manifest =
            crate::config::AppManifest::load_from_path(&path).expect("manifest should reload");
        assert!(
            !final_manifest
                .apps
                .iter()
                .any(|app| app.id == "sevenzip-e2e")
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn e2e_edit_existing_generated_app_into_real_winget_app() {
        let manifest =
            crate::config::AppManifest::load_from_default_path().expect("apps example should load");
        let placeholder = super::AppEntry {
            id: "new-app-edit-e2e".to_owned(),
            name: "新软件 编辑 E2E".to_owned(),
            category: "utility".to_owned(),
            homepage_url: None,
            enabled_by_default: false,
            verification_status: "candidate_medium".to_owned(),
            source: super::PackageSource {
                source_type: "direct_url".to_owned(),
                package_id: None,
                url: Some("https://example.com/installer.exe".to_owned()),
                repo: None,
                asset_pattern: None,
            },
            install: super::InstallSpec {
                method: "direct_exe".to_owned(),
                requires_admin: true,
                supports_custom_path: false,
                args: None,
                silent_args: Some(vec!["/S".to_owned()]),
                direct_silent_args: Some(vec!["/S".to_owned()]),
                direct_install_location_arg: None,
                fallback_notes: Some("test placeholder".to_owned()),
            },
            detect: super::DetectSpec {
                detect_type: "path_exists".to_owned(),
                rules: vec![super::DetectRule {
                    rule_type: "path_exists".to_owned(),
                    value: "C:\\Program Files\\Placeholder\\app.exe".to_owned(),
                }],
            },
            notes: Some("test placeholder".to_owned()),
        };
        let with_placeholder =
            super::manifest_with_saved_app(&manifest, None, placeholder).expect("add placeholder");
        let form = super::AddAppFormState {
            visible: true,
            editing_existing_id: Some("new-app-edit-e2e".to_owned()),
            id: "new-app-edit-e2e".to_owned(),
            name: "7-Zip Edited E2E".to_owned(),
            category: "utility".to_owned(),
            winget_id: "7zip.7zip".to_owned(),
            homepage: "https://www.7-zip.org/".to_owned(),
            detect_path: "C:\\Program Files\\7-Zip\\7zFM.exe".to_owned(),
        };
        let app = super::app_from_add_form(&form).expect("edit form should build");
        let edited_manifest = super::manifest_with_saved_app(
            &with_placeholder,
            form.editing_existing_id.as_deref(),
            app,
        )
        .expect("placeholder should update");

        let edited = edited_manifest
            .apps
            .iter()
            .find(|app| app.id == "new-app-edit-e2e")
            .expect("edited app should exist");
        assert_eq!(edited.name, "7-Zip Edited E2E");
        assert_eq!(edited.source.source_type, "winget");
        assert_eq!(edited.source.package_id.as_deref(), Some("7zip.7zip"));
        assert_eq!(edited.install.method, "winget");
        assert_eq!(
            edited.detect.rules[0].value,
            "C:\\Program Files\\7-Zip\\7zFM.exe"
        );
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
