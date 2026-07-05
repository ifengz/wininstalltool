use std::{collections::HashSet, fs};

#[test]
fn apps_example_is_valid_json_with_unique_ids() {
    let data = fs::read_to_string("config/apps.example.json").expect("apps example exists");
    let value: serde_json::Value = serde_json::from_str(&data).expect("valid json");
    let apps = value["apps"].as_array().expect("apps is an array");
    let mut ids = HashSet::new();

    assert_eq!(value["schema_version"], 1);
    assert_eq!(apps.len(), 10);

    for app in apps {
        let id = app["id"].as_str().expect("app id is string");
        assert!(ids.insert(id.to_string()), "duplicate app id: {id}");
    }
}
