use crate::config::{AppEntry, AppManifest, DetectRule};
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct InstallPlan {
    pub selected_count: usize,
    pub requires_admin_count: usize,
    pub needs_verification_count: usize,
}

pub fn build_plan(manifest: &AppManifest, selected_ids: &[String]) -> InstallPlan {
    let selected = manifest
        .apps
        .iter()
        .filter(|app| selected_ids.iter().any(|id| id == &app.id));

    selected.fold(InstallPlan::default(), |mut plan, app| {
        plan.selected_count += 1;

        if app.install.requires_admin {
            plan.requires_admin_count += 1;
        }

        if !app.verification_status.ends_with("_high") {
            plan.needs_verification_count += 1;
        }

        plan
    })
}

#[derive(Debug, Default)]
pub struct ConfigValidationReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DetectionState {
    Installed,
    NotInstalled,
    Unsupported,
    Error,
}

#[derive(Clone, Debug)]
pub struct DetectionReport {
    pub app_id: String,
    pub app_name: String,
    pub state: DetectionState,
    pub details: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallCommand {
    pub app_id: String,
    pub app_name: String,
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandResult {
    pub app_id: String,
    pub app_name: String,
    pub command: String,
    pub exit_code: Option<i32>,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

pub fn build_install_command(
    app: &AppEntry,
    install_root: &str,
    cache_root: &Path,
) -> Result<InstallCommand, String> {
    match app.install.method.as_str() {
        "winget" | "winget_if_missing" => winget_install_command(app),
        "msi" => msi_install_command(app, cache_root),
        "direct_exe" => direct_exe_install_command(app, install_root, cache_root),
        method => Err(format!(
            "{} uses unsupported install method `{method}`",
            app.name
        )),
    }
}

pub fn run_install_command(command: &InstallCommand) -> CommandResult {
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new(&command.program)
            .args(&command.args)
            .output();

        return match output {
            Ok(output) => CommandResult {
                app_id: command.app_id.clone(),
                app_name: command.app_name.clone(),
                command: command.render(),
                exit_code: output.status.code(),
                success: output.status.success(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            },
            Err(error) => CommandResult {
                app_id: command.app_id.clone(),
                app_name: command.app_name.clone(),
                command: command.render(),
                exit_code: None,
                success: false,
                stdout: String::new(),
                stderr: error.to_string(),
            },
        };
    }

    #[cfg(not(target_os = "windows"))]
    {
        CommandResult {
            app_id: command.app_id.clone(),
            app_name: command.app_name.clone(),
            command: command.render(),
            exit_code: None,
            success: false,
            stdout: String::new(),
            stderr: "real installation is only supported on Windows".to_owned(),
        }
    }
}

impl InstallCommand {
    pub fn render(&self) -> String {
        std::iter::once(self.program.as_str())
            .chain(self.args.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn winget_install_command(app: &AppEntry) -> Result<InstallCommand, String> {
    let args = app
        .install
        .args
        .clone()
        .ok_or_else(|| format!("{} missing winget args", app.name))?;

    Ok(InstallCommand {
        app_id: app.id.clone(),
        app_name: app.name.clone(),
        program: "winget".to_owned(),
        args,
    })
}

fn msi_install_command(app: &AppEntry, cache_root: &Path) -> Result<InstallCommand, String> {
    let installer = find_cached_installer(&app.id, cache_root, &["msi"])?;
    let mut args = vec!["/i".to_owned(), installer.to_string_lossy().into_owned()];
    args.extend(app.install.args.clone().unwrap_or_default());

    Ok(InstallCommand {
        app_id: app.id.clone(),
        app_name: app.name.clone(),
        program: "msiexec".to_owned(),
        args,
    })
}

fn direct_exe_install_command(
    app: &AppEntry,
    install_root: &str,
    cache_root: &Path,
) -> Result<InstallCommand, String> {
    let installer = find_cached_installer(&app.id, cache_root, &["exe"])?;
    let mut args = app
        .install
        .direct_silent_args
        .clone()
        .or_else(|| app.install.silent_args.clone())
        .unwrap_or_default();

    if app.install.supports_custom_path {
        if let Some(template) = app.install.direct_install_location_arg.as_deref() {
            args.push(template.replace("{install_dir}", install_root));
        }
    }

    Ok(InstallCommand {
        app_id: app.id.clone(),
        app_name: app.name.clone(),
        program: installer.to_string_lossy().into_owned(),
        args,
    })
}

fn find_cached_installer(
    app_id: &str,
    cache_root: &Path,
    extensions: &[&str],
) -> Result<PathBuf, String> {
    let app_cache = cache_root.join(app_id);
    let entries = std::fs::read_dir(&app_cache)
        .map_err(|_| format!("cache installer not found under {}", app_cache.display()))?;

    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| {
                        extensions
                            .iter()
                            .any(|expected| extension.eq_ignore_ascii_case(expected))
                    })
        })
        .ok_or_else(|| format!("cache installer not found under {}", app_cache.display()))
}

pub fn validate_manifest_for_install(manifest: &AppManifest) -> ConfigValidationReport {
    let mut report = ConfigValidationReport::default();

    if manifest.default_install_root.trim().is_empty() {
        report
            .errors
            .push("default_install_root cannot be empty".to_owned());
    }

    for app in &manifest.apps {
        if app.id.trim().is_empty() {
            report.errors.push("app id cannot be empty".to_owned());
        }
        if app.name.trim().is_empty() {
            report
                .errors
                .push(format!("app `{}` name cannot be empty", app.id));
        }
        if app.install.method.trim().is_empty() {
            report
                .errors
                .push(format!("app `{}` install.method cannot be empty", app.id));
        }
        if app.detect.rules.is_empty() {
            report
                .errors
                .push(format!("app `{}` must define detect.rules", app.id));
        }

        validate_source(app, &mut report);
    }

    report
}

pub fn detect_selected_apps(
    manifest: &AppManifest,
    selected_ids: &[String],
) -> Vec<DetectionReport> {
    manifest
        .apps
        .iter()
        .filter(|app| selected_ids.iter().any(|id| id == &app.id))
        .map(detect_app)
        .collect()
}

pub fn detect_app(app: &AppEntry) -> DetectionReport {
    let mut unsupported_count = 0usize;
    let mut checked_count = 0usize;
    let mut last_error = None;

    for rule in &app.detect.rules {
        match evaluate_detect_rule(rule) {
            RuleEvaluation::Matched => {
                return DetectionReport {
                    app_id: app.id.clone(),
                    app_name: app.name.clone(),
                    state: DetectionState::Installed,
                    details: format!("matched {}", rule.rule_type),
                };
            }
            RuleEvaluation::NotMatched => {
                checked_count += 1;
            }
            RuleEvaluation::Unsupported => {
                unsupported_count += 1;
            }
            RuleEvaluation::Error(error) => {
                last_error = Some(error);
            }
        }
    }

    if let Some(error) = last_error {
        DetectionReport {
            app_id: app.id.clone(),
            app_name: app.name.clone(),
            state: DetectionState::Error,
            details: error,
        }
    } else if checked_count > 0 {
        DetectionReport {
            app_id: app.id.clone(),
            app_name: app.name.clone(),
            state: DetectionState::NotInstalled,
            details: "no detection rule matched".to_owned(),
        }
    } else if unsupported_count > 0 {
        DetectionReport {
            app_id: app.id.clone(),
            app_name: app.name.clone(),
            state: DetectionState::Unsupported,
            details: "detection requires Windows registry support".to_owned(),
        }
    } else {
        DetectionReport {
            app_id: app.id.clone(),
            app_name: app.name.clone(),
            state: DetectionState::Unsupported,
            details: "no supported detection rules".to_owned(),
        }
    }
}

fn validate_source(app: &AppEntry, report: &mut ConfigValidationReport) {
    match app.source.source_type.as_str() {
        "winget" | "preinstalled_or_winget" => {
            if app
                .source
                .package_id
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
            {
                report.errors.push(format!(
                    "app `{}` source.package_id cannot be empty",
                    app.id
                ));
            }
        }
        "direct_url" => {
            if app.source.url.as_deref().unwrap_or("").trim().is_empty() {
                report
                    .errors
                    .push(format!("app `{}` source.url cannot be empty", app.id));
            }
        }
        "github_release" => {
            if app.source.repo.as_deref().unwrap_or("").trim().is_empty() {
                report
                    .errors
                    .push(format!("app `{}` source.repo cannot be empty", app.id));
            }
        }
        source_type => report.warnings.push(format!(
            "app `{}` uses unrecognized source type `{}`",
            app.id, source_type
        )),
    }
}

enum RuleEvaluation {
    Matched,
    NotMatched,
    Unsupported,
    #[allow(dead_code)]
    Error(String),
}

fn evaluate_detect_rule(rule: &DetectRule) -> RuleEvaluation {
    match rule.rule_type.as_str() {
        "path_exists" => {
            let path = expand_windows_env_path(&rule.value);
            if Path::new(&path).exists() {
                RuleEvaluation::Matched
            } else {
                RuleEvaluation::NotMatched
            }
        }
        "registry_uninstall_display_name_contains" | "registry_uninstall_product_code" => {
            evaluate_registry_rule(rule)
        }
        _ => RuleEvaluation::Unsupported,
    }
}

fn expand_windows_env_path(value: &str) -> String {
    let mut expanded = value.to_owned();
    for (name, fallback) in [
        ("ProgramFiles", "C:\\Program Files"),
        ("ProgramFiles(x86)", "C:\\Program Files (x86)"),
        ("LocalAppData", ""),
        ("AppData", ""),
    ] {
        let token = format!("%{}%", name);
        if expanded.contains(&token) {
            let replacement = std::env::var(name).unwrap_or_else(|_| fallback.to_owned());
            expanded = expanded.replace(&token, &replacement);
        }
    }
    expanded
}

#[cfg(target_os = "windows")]
fn evaluate_registry_rule(rule: &DetectRule) -> RuleEvaluation {
    let output = std::process::Command::new("reg")
        .args([
            "query",
            "HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
            "/s",
            "/f",
            &rule.value,
        ])
        .output();

    match output {
        Ok(output) if output.status.success() => RuleEvaluation::Matched,
        Ok(_) => RuleEvaluation::NotMatched,
        Err(error) => RuleEvaluation::Error(error.to_string()),
    }
}

#[cfg(not(target_os = "windows"))]
fn evaluate_registry_rule(_rule: &DetectRule) -> RuleEvaluation {
    RuleEvaluation::Unsupported
}

#[cfg(test)]
mod tests {
    use super::{
        DetectionState, build_install_command, build_plan, detect_app,
        validate_manifest_for_install,
    };
    use crate::config::{
        AppEntry, AppManifest, DetectRule, DetectSpec, InstallSpec, PackageSource,
    };

    #[test]
    fn plan_counts_selected_apps() {
        let manifest = AppManifest::load_from_default_path().expect("apps example should load");
        let selected = vec!["chrome".to_string(), "edge".to_string()];
        let plan = build_plan(&manifest, &selected);

        assert_eq!(plan.selected_count, 2);
        assert!(plan.requires_admin_count >= 1);
    }

    #[test]
    fn validation_accepts_apps_example_manifest() {
        let manifest = AppManifest::load_from_default_path().expect("apps example should load");
        let report = validate_manifest_for_install(&manifest);

        assert!(
            report.errors.is_empty(),
            "apps example should not have config errors: {:?}",
            report.errors
        );
    }

    #[test]
    fn path_exists_rule_detects_existing_file() {
        let marker_path = std::env::temp_dir().join("wininstalltool-detect-marker.txt");
        std::fs::write(&marker_path, "ok").expect("marker should be writable");
        let app = test_app_with_rules(vec![DetectRule {
            rule_type: "path_exists".to_owned(),
            value: marker_path.to_string_lossy().into_owned(),
        }]);

        let report = detect_app(&app);

        let _ = std::fs::remove_file(marker_path);
        assert_eq!(report.state, DetectionState::Installed);
    }

    #[test]
    fn unsupported_rules_report_unsupported_detection() {
        let app = test_app_with_rules(vec![DetectRule {
            rule_type: "registry_uninstall_display_name_contains".to_owned(),
            value: "Example".to_owned(),
        }]);

        let report = detect_app(&app);

        #[cfg(not(target_os = "windows"))]
        assert_eq!(report.state, DetectionState::Unsupported);
    }

    #[test]
    fn winget_command_uses_manifest_args() {
        let manifest = AppManifest::load_from_default_path().expect("apps example should load");
        let chrome = manifest
            .apps
            .iter()
            .find(|app| app.id == "chrome")
            .expect("chrome should exist");

        let command = build_install_command(chrome, "D:\\Apps", std::env::temp_dir().as_path())
            .expect("winget command should build");

        assert_eq!(command.program, "winget");
        assert!(command.args.iter().any(|arg| arg == "Google.Chrome"));
    }

    #[test]
    fn msi_command_requires_cached_installer() {
        let manifest = AppManifest::load_from_default_path().expect("apps example should load");
        let notepadpp = manifest
            .apps
            .iter()
            .find(|app| app.id == "notepadpp")
            .expect("notepad++ should exist");

        let error = build_install_command(notepadpp, "D:\\Apps", std::env::temp_dir().as_path())
            .expect_err("missing cache should fail loudly");

        assert!(error.contains("cache"));
    }

    #[test]
    fn direct_exe_command_uses_cached_installer_and_silent_args() {
        let cache_root =
            std::env::temp_dir().join(format!("wininstalltool-cache-test-{}", std::process::id()));
        let app_cache = cache_root.join("example");
        std::fs::create_dir_all(&app_cache).expect("cache dir should be writable");
        let installer = app_cache.join("example.exe");
        std::fs::write(&installer, "fake exe").expect("installer marker should be writable");
        let app = test_app_with_rules(vec![DetectRule {
            rule_type: "path_exists".to_owned(),
            value: installer.to_string_lossy().into_owned(),
        }]);

        let command =
            build_install_command(&app, "D:\\Apps", &cache_root).expect("direct exe should build");

        let _ = std::fs::remove_dir_all(cache_root);
        assert_eq!(command.program, installer.to_string_lossy());
        assert_eq!(command.args, vec!["/S"]);
    }

    fn test_app_with_rules(rules: Vec<DetectRule>) -> AppEntry {
        AppEntry {
            id: "example".to_owned(),
            name: "Example".to_owned(),
            category: "utility".to_owned(),
            homepage_url: None,
            enabled_by_default: true,
            verification_status: "candidate_high".to_owned(),
            source: PackageSource {
                source_type: "direct_url".to_owned(),
                package_id: None,
                url: Some("https://example.com/app.exe".to_owned()),
                repo: None,
                asset_pattern: None,
            },
            install: InstallSpec {
                method: "direct_exe".to_owned(),
                requires_admin: false,
                supports_custom_path: false,
                args: None,
                silent_args: Some(vec!["/S".to_owned()]),
                direct_silent_args: None,
                direct_install_location_arg: None,
                fallback_notes: None,
            },
            detect: DetectSpec {
                detect_type: "any".to_owned(),
                rules,
            },
            notes: None,
        }
    }
}
