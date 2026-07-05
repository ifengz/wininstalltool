use std::{collections::HashSet, fs};

#[test]
fn apps_example_is_valid_json_with_unique_ids() {
    let data = fs::read_to_string("config/apps.example.json").expect("apps example exists");
    let value: serde_json::Value = serde_json::from_str(&data).expect("valid json");
    let apps = value["apps"].as_array().expect("apps is an array");
    let mut ids = HashSet::new();

    assert_eq!(value["schema_version"], 1);
    assert!(!apps.is_empty(), "apps should not be empty");

    for app in apps {
        let id = app["id"].as_str().expect("app id is string");
        assert!(ids.insert(id.to_string()), "duplicate app id: {id}");
    }
}

#[test]
fn apps_example_contains_full_requested_company_list() {
    let data = fs::read_to_string("config/apps.example.json").expect("apps example exists");
    let value: serde_json::Value = serde_json::from_str(&data).expect("valid json");
    let apps = value["apps"].as_array().expect("apps is an array");
    let names = apps
        .iter()
        .map(|app| app["name"].as_str().expect("app name is string"))
        .collect::<HashSet<_>>();

    let expected_names = [
        "紫鸟超级浏览器",
        "钉钉",
        "微信",
        "微信输入法",
        "搜狗输入法",
        "WPS Office",
        "Google Chrome",
        "Microsoft Edge",
        "Hubstudio",
        "向日葵远程",
        "ToDesk",
        "UU 远程",
        "火绒安全",
        "夸克网盘",
        "百度网盘",
        "网易云音乐",
        "QQ 音乐",
        "Codex 桌面客户端",
        "Claude Code 桌面客户端",
        "Zcode 客户端",
        "飞书",
        "QQ",
        "Notepad++",
        "Honeyview",
        "Vivaldi",
        "Ente Auth",
        "CC Switch",
        "Clash Verge",
        "360 极速浏览器",
        "360 浏览器",
        "PixPin 截图",
    ];

    for expected_name in expected_names {
        assert!(
            names.contains(expected_name),
            "missing requested app: {expected_name}"
        );
    }
}

#[test]
fn apps_example_uses_verified_winget_sources_where_available() {
    let data = fs::read_to_string("config/apps.example.json").expect("apps example exists");
    let value: serde_json::Value = serde_json::from_str(&data).expect("valid json");
    let apps = value["apps"].as_array().expect("apps is an array");
    let apps_by_id = apps
        .iter()
        .map(|app| (app["id"].as_str().expect("app id is string"), app))
        .collect::<std::collections::HashMap<_, _>>();

    let expected_winget_ids = [
        ("uu_remote", "NetEase.UURemote"),
        ("quark_netdisk", "Alibaba.QuarkCloudDrive"),
        ("codex_desktop", "OpenAI.Codex"),
        ("claude_code_desktop", "Anthropic.ClaudeCode"),
        ("zcode_client", "ZhipuAI.ZCode"),
        ("browser_360_extreme", "360.360Chrome"),
        ("browser_360", "360.360SE"),
    ];

    for (app_id, package_id) in expected_winget_ids {
        let app = apps_by_id
            .get(app_id)
            .unwrap_or_else(|| panic!("missing app id: {app_id}"));
        assert_eq!(app["source"]["type"], "winget", "{app_id} source type");
        assert_eq!(
            app["source"]["package_id"], package_id,
            "{app_id} package id"
        );
        assert_eq!(
            app["install"]["method"], "winget",
            "{app_id} install method"
        );
    }
}
