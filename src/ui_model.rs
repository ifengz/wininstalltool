use crate::config::{AppEntry, AppManifest};
use crate::engine::build_plan;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallerViewModel {
    pub selected_count: usize,
    pub admin_count: usize,
    pub needs_verification_count: usize,
    pub categories: Vec<CategoryView>,
    pub rows: Vec<AppRowView>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CategoryView {
    pub id: Option<String>,
    pub label: String,
    pub count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppRowView {
    pub selected: bool,
    pub name: String,
    pub purpose: String,
    pub source: String,
    pub homepage_url: String,
    pub verification: String,
    pub install_method: String,
    pub path: String,
}

pub fn build_view_model(
    manifest: &AppManifest,
    selected_ids: &[String],
    active_category: Option<&str>,
    install_root: &str,
) -> InstallerViewModel {
    let plan = build_plan(manifest, selected_ids);
    let rows = manifest
        .apps
        .iter()
        .filter(|app| active_category.is_none_or(|category| app.category == category))
        .map(|app| app_row_view(app, selected_ids, install_root))
        .collect();

    InstallerViewModel {
        selected_count: plan.selected_count,
        admin_count: plan.requires_admin_count,
        needs_verification_count: plan.needs_verification_count,
        categories: category_views(manifest),
        rows,
    }
}

fn app_row_view(app: &AppEntry, selected_ids: &[String], install_root: &str) -> AppRowView {
    AppRowView {
        selected: selected_ids.iter().any(|id| id == &app.id),
        name: app.name.clone(),
        purpose: category_label(&app.category).to_owned(),
        source: source_summary(app),
        homepage_url: app.homepage_url.clone().unwrap_or_default(),
        verification: verification_label(&app.verification_status).to_owned(),
        install_method: app.install.method.clone(),
        path: install_path_label(app, install_root),
    }
}

fn category_views(manifest: &AppManifest) -> Vec<CategoryView> {
    let mut categories = vec![CategoryView {
        id: None,
        label: "全部".to_owned(),
        count: manifest.apps.len(),
    }];

    categories.extend(ordered_categories(manifest).into_iter().map(|category| {
        CategoryView {
            id: Some(category.to_owned()),
            label: category_label(category).to_owned(),
            count: manifest
                .apps
                .iter()
                .filter(|app| app.category == category)
                .count(),
        }
    }));

    categories
}

fn ordered_categories(manifest: &AppManifest) -> Vec<&str> {
    [
        "browser",
        "messaging",
        "office",
        "developer_tool",
        "security",
        "image_viewer",
    ]
    .into_iter()
    .filter(|category| manifest.apps.iter().any(|app| app.category == *category))
    .collect()
}

fn category_label(category: &str) -> &'static str {
    match category {
        "browser" => "浏览器",
        "messaging" => "沟通协作",
        "developer_tool" => "开发工具",
        "office" => "办公套件",
        "security" => "安全防护",
        "image_viewer" => "看图工具",
        _ => "通用软件",
    }
}

fn source_summary(app: &AppEntry) -> String {
    match app.source.source_type.as_str() {
        "winget" => app
            .source
            .package_id
            .as_ref()
            .map(|package_id| format!("winget / {package_id}"))
            .unwrap_or_else(|| "winget".to_owned()),
        "preinstalled_or_winget" => app
            .source
            .package_id
            .as_ref()
            .map(|package_id| format!("已预装或 winget / {package_id}"))
            .unwrap_or_else(|| "已预装或 winget".to_owned()),
        "direct_url" => "官网直链".to_owned(),
        "github_release" => app
            .source
            .repo
            .as_ref()
            .map(|repo| format!("GitHub Release / {repo}"))
            .unwrap_or_else(|| "GitHub Release".to_owned()),
        source_type => source_type.to_owned(),
    }
}

fn verification_label(status: &str) -> &'static str {
    match status {
        "candidate_high" => "参数已确认",
        "candidate_medium" => "待实机验证",
        _ => "待确认",
    }
}

fn install_path_label(app: &AppEntry, install_root: &str) -> String {
    if app.install.supports_custom_path {
        install_root.to_owned()
    } else {
        "由安装器决定".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use crate::config::AppManifest;

    #[test]
    fn view_model_preserves_selected_counts_and_categories() {
        let manifest = AppManifest::load_from_default_path().expect("manifest should load");
        let selected = manifest
            .apps
            .iter()
            .filter(|app| app.enabled_by_default)
            .map(|app| app.id.clone())
            .collect::<Vec<_>>();

        let view = super::build_view_model(&manifest, &selected, None, "D:\\CompanyApps");

        assert_eq!(view.selected_count, 10);
        assert_eq!(view.admin_count, 9);
        assert_eq!(view.needs_verification_count, 6);
        assert_eq!(view.categories[0].label, "全部");
        assert_eq!(view.categories[0].count, 10);
        assert!(
            view.categories
                .iter()
                .any(|category| category.label == "浏览器")
        );
        assert_eq!(view.rows.len(), 10);
    }

    #[test]
    fn view_model_uses_custom_root_only_for_supported_apps() {
        let manifest = AppManifest::load_from_default_path().expect("manifest should load");
        let selected = manifest
            .apps
            .iter()
            .map(|app| app.id.clone())
            .collect::<Vec<_>>();

        let view =
            super::build_view_model(&manifest, &selected, Some("browser"), "D:\\CompanyApps");

        let chrome = view
            .rows
            .iter()
            .find(|row| row.name == "Google Chrome")
            .unwrap();
        let vivaldi = view.rows.iter().find(|row| row.name == "Vivaldi").unwrap();

        assert_eq!(chrome.path, "由安装器决定");
        assert_eq!(vivaldi.path, "D:\\CompanyApps");
    }
}
