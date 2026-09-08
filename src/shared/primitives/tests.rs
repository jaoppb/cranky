#[cfg(test)]
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::{
    ChildSizesMap, DynamicValue, ModuleInstanceId, ModuleKey, ModuleName, ModuleOptions,
};
use std::collections::HashMap;

#[test]
fn test_dynamic_value_and_module_options() {
    let mut map = HashMap::new();
    map.insert("string_key".to_string(), DynamicValue::from("hello"));
    map.insert("bool_key".to_string(), DynamicValue::from(true));
    map.insert("num_key".to_string(), DynamicValue::from(42.0));
    map.insert(
        "arr_key".to_string(),
        DynamicValue::from(vec![DynamicValue::from("item1")]),
    );

    let options = ModuleOptions::new(map);
    assert_eq!(options.get("string_key").unwrap().as_str(), Some("hello"));
    assert_eq!(options.get("bool_key").unwrap().as_bool(), Some(true));
    assert_eq!(options.get("num_key").unwrap().as_f64(), Some(42.0));
    assert_eq!(options.get("num_key").unwrap().as_i64(), Some(42));
    assert_eq!(options.get("arr_key").unwrap().as_array().unwrap().len(), 1);
    assert!(!options.is_empty());
}

#[test]
fn test_module_key_and_child_sizes() {
    let key1 = ModuleKey::new(
        ModuleName::new("workspace"),
        Some(ModuleInstanceId::new("ws1")),
    );
    let key2 = ModuleKey::from_name("clock");

    assert_eq!(key1.to_string(), "workspace:ws1");
    assert_eq!(key2.to_string(), "clock");

    let mut sizes = ChildSizesMap::new();
    sizes.insert(key1.clone(), Size::new(100, 30));
    sizes.insert(key2, Size::new(80, 25));

    assert_eq!(sizes.get(&key1), Some(&Size::new(100, 30)));
    assert_eq!(
        sizes.get_by_name_or_key(&ModuleName::new("clock"), None),
        Some(&Size::new(80, 25))
    );
    assert_eq!(
        sizes.get_by_name_or_key(
            &ModuleName::new("workspace"),
            Some(&ModuleInstanceId::new("ws1"))
        ),
        Some(&Size::new(100, 30))
    );
}

#[test]
fn test_script_monitor_info_builder() {
    let id = crate::shared::primitives::MonitorId::new("DP-2");
    let info = crate::shared::primitives::ScriptMonitorInfo::from_id(&id)
        .with_name("DisplayPort-2".to_string())
        .with_size(Size::new(2560, 1440))
        .with_scale(crate::shared::primitives::Scale::new(2.0))
        .with_focused(true)
        .with_workspaces(Some(3), Some(4));

    assert_eq!(info.id(), &id);
    assert_eq!(info.name(), "DisplayPort-2");
    assert_eq!(info.size(), Size::new(2560, 1440));
    assert_eq!(info.scale(), crate::shared::primitives::Scale::new(2.0));
    assert!(info.is_focused());
    assert_eq!(info.active_workspace_id(), Some(3));
    assert_eq!(info.special_workspace_id(), Some(4));
}
