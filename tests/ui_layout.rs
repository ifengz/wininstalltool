use std::fs;

#[test]
fn main_content_panels_keep_right_inset() {
    let ui = fs::read_to_string("ui/main.slint")
        .expect("main Slint UI exists")
        .replace("\r\n", "\n");
    let right_inset_count = ui.matches("padding-right: 28px;").count();
    let clipping_panel_count = ui.matches("clip: true;").count();
    let stretching_clipped_panel_count = ui
        .matches("clip: true;\n                horizontal-stretch: 1;")
        .count();

    assert!(
        ui.contains("width: root.width;") && ui.contains("height: root.height;"),
        "root layout must fill the window so card boundaries track the window edge"
    );
    assert!(
        ui.contains("text: \"Sinopin 软件安装\";") && ui.contains("text: \"软件列表\";"),
        "primary UI titles must use the approved product wording"
    );
    assert!(
        ui.contains("text: \"软件列表\";\n                            font-size: 13px;")
            && ui.contains("text: \"状态 / 日志\";\n                            font-size: 13px;"),
        "software-list and status/log section titles must use the same font size"
    );
    assert!(
        !ui.contains("Office Standard") && !ui.contains("安装中心"),
        "old placeholder titles must not return"
    );
    assert!(
        !ui.contains("10 个优先软件，顺序执行并写日志"),
        "sidebar should not show the removed priority-software subtitle"
    );
    assert!(
        ui.contains("x: 16px;")
            && ui.contains("y: 16px;")
            && ui.contains("width: 246px;")
            && ui.contains("height: root.height - 32px;"),
        "sidebar must keep the approved fixed geometry"
    );
    assert!(
        ui.contains("x: 274px;")
            && ui.contains("y: 16px;")
            && ui.contains("width: root.width - 290px;")
            && ui.contains("height: root.height - 32px;"),
        "right column must use the approved explicit geometry instead of preferred content width"
    );
    assert!(
        right_inset_count >= 2,
        "software list and status panels must keep explicit right-side inset"
    );
    assert!(
        clipping_panel_count >= 2,
        "software list and status panels must clip oversized child content"
    );
    assert!(
        stretching_clipped_panel_count >= 2,
        "right-side cards must stretch horizontally to the available column width"
    );
    assert!(
        ui.contains("height: root.delete-confirm-visible ? 28px : 0px;"),
        "hidden delete-confirm row must not push the log editor down"
    );
    assert!(
        ui.contains("height: root.add-form-visible ? 58px : 0px;"),
        "hidden add form must not push the log editor down"
    );
    assert!(
        ui.contains("min-height: 96px;") && ui.contains("vertical-stretch: 1;"),
        "log TextEdit must stretch from the top of the status panel with a usable minimum height"
    );
    for section in ["执行", "软件管理", "配置工具"] {
        assert!(
            ui.contains(&format!("SidebarSectionLabel {{ label: \"{section}\"; }}")),
            "sidebar action buttons must keep the {section} section"
        );
    }
    for config_button in ["检查清单", "扫描已安装", "打开清单文件", "重新读取清单"]
    {
        assert!(
            ui.contains(&format!("text: \"{config_button}\";")),
            "config tool button must use plain Chinese wording: {config_button}"
        );
    }
    for old_config_button in ["校验配置", "检测已安装", "打开软件配置", "重载配置"]
    {
        assert!(
            !ui.contains(&format!("text: \"{old_config_button}\";")),
            "engineering config wording should not be shown in the sidebar: {old_config_button}"
        );
    }
    assert!(
        !ui.contains("width: 76px;\n                            height: 26px;"),
        "row action buttons should not live in the category row because they push cards right"
    );
    assert!(
        !ui.contains("text: \"切换选择\";"),
        "selection should be done by clicking the table checkbox column, not a sidebar button"
    );
    assert!(
        ui.contains("CheckBox {")
            && ui.contains("checked: root.all-visible-selected;")
            && ui.contains("toggled => { root.toggle-visible-selection(); }"),
        "the selection header must be a real checkbox wired to visible-list selection"
    );
    assert!(
        !ui.contains("Text { text: \"选\""),
        "the selection header must not regress to a plain text label"
    );
    assert!(
        ui.contains("selected: bool,"),
        "row selection state must be boolean so it can drive a real checkbox"
    );
    assert!(
        ui.contains("checked: row.selected;")
            && ui.contains("toggled => { root.toggle-row-selection(index); }"),
        "each software row must use a real checkbox for selection"
    );
    assert!(
        !ui.contains("Text { text: row.selected;"),
        "row selection must not be rendered as a checkmark text glyph"
    );
    assert!(
        ui.contains("placeholder-text: \"分类；新名称会新增\""),
        "add-software form must keep a clear category field instead of exposing internal English defaults"
    );
    let row_loop_index = ui
        .find("for row[index] in root.app-rows")
        .expect("software row loop should exist");
    let fixed_header_index = ui
        .find("// FixedTableHeaderGuard")
        .expect("software table must mark its fixed header block");
    let row_scroll_index = ui
        .find("// TableRowsVerticalScrollGuard")
        .expect("software rows must mark their vertical scroll block");
    assert!(
        fixed_header_index < row_scroll_index && row_scroll_index < row_loop_index,
        "software table header must stay outside the vertical row ScrollView"
    );
    let category_loop_index = ui
        .find("for category[index] in root.category-labels")
        .expect("category tabs should exist");
    let category_scroll_index = ui
        .find("// CategoryTabsScrollGuard")
        .expect("category tabs need a guarded scroll container");
    assert!(
        category_scroll_index < category_loop_index,
        "category tabs must live inside a horizontal ScrollView so expanded categories do not overflow"
    );
    assert!(
        ui.contains("width: root.width - 350px;")
            && ui.contains(
                "viewport-width: max(root.width - 350px, root.category-labels.length * 118px);"
            )
            && ui.contains("horizontal-scrollbar-policy: always-on;"),
        "category tabs must have a bounded visible width and an explicit wider viewport for for-loop content"
    );
    for removed_sidebar_button in ["打开官网", "编辑当前软件", "删除当前软件"] {
        assert!(
            !ui.contains(&format!("text: \"{removed_sidebar_button}\";")),
            "{removed_sidebar_button} should be exposed as row-hover actions, not sidebar buttons"
        );
    }
    for row_action in ["官网", "编辑", "删除"] {
        assert!(
            ui.contains(&format!("text: \"{row_action}\";")),
            "row hover action {row_action} must remain available"
        );
    }
    assert!(
        ui.contains("component RowActionButton inherits Rectangle"),
        "row actions must use the centered RowActionButton component"
    );
    assert!(
        !ui.contains("ProgressIndicator"),
        "status panel should keep only the bottom log progress indicator"
    );
    assert!(
        ui.contains("horizontal-alignment: center;") && ui.contains("vertical-alignment: center;"),
        "row action button text must be horizontally and vertically centered"
    );
    assert!(
        ui.contains("opacity: row_hover.has-hover || root.current-row == index ? 1 : 0;"),
        "row hover actions must keep their layout slot and fade instead of toggling visibility"
    );
    assert!(
        !ui.contains("visible: row_hover.has-hover || root.current-row == index;"),
        "row hover actions must not toggle visibility because it causes flicker and alignment shifts"
    );
    assert!(
        ui.contains("height: 20px;"),
        "phase rows must stay compact so the driver-install step does not overflow"
    );
    assert!(
        ui.contains("spacing: 6px;"),
        "sidebar layout must stay compact enough for all action groups and phase rows"
    );
    assert!(
        ui.matches("height: 28px;").count() >= 7,
        "sidebar action buttons must stay compact enough to avoid bottom overflow"
    );
}

#[test]
fn windows_packaged_app_uses_gui_subsystem_and_software_renderer() {
    let main_rs = fs::read_to_string("src/main.rs")
        .expect("main Rust entry exists")
        .replace("\r\n", "\n");

    assert!(
        main_rs.contains("#![cfg_attr(target_os = \"windows\", windows_subsystem = \"windows\")]"),
        "Windows release app must not open a separate console window"
    );
    assert!(
        main_rs.contains("fn configure_windows_renderer() -> Result<(), slint::PlatformError>")
            && main_rs.contains(".backend_name(\"winit\".to_owned())")
            && main_rs.contains(".renderer_name(\"software\".to_owned())")
            && main_rs.contains(".select()"),
        "Windows startup must select Slint's software renderer before creating the window"
    );
    assert!(
        main_rs.find("configure_windows_renderer()?") < main_rs.find("let app = AppWindow::new()?"),
        "renderer selection must happen before AppWindow::new()"
    );
}
