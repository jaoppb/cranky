use super::*;

#[test]
fn test_margin_dto_all() {
    let dto = MarginConfigDto::All(10);
    let domain = dto.into_domain();
    assert_eq!(domain.top().value(), 10);
    assert_eq!(domain.bottom().value(), 10);
    assert_eq!(domain.left().value(), 10);
    assert_eq!(domain.right().value(), 10);
}

#[test]
fn test_margin_dto_fields_horizontal_vertical() {
    let dto = MarginConfigDto::Fields {
        top: None,
        bottom: None,
        left: None,
        right: None,
        horizontal: Some(15),
        vertical: Some(25),
    };
    let domain = dto.into_domain();
    assert_eq!(domain.top().value(), 25);
    assert_eq!(domain.bottom().value(), 25);
    assert_eq!(domain.left().value(), 15);
    assert_eq!(domain.right().value(), 15);
}

#[test]
fn test_margin_dto_fields_override() {
    let dto = MarginConfigDto::Fields {
        top: Some(5),
        bottom: None,
        left: Some(8),
        right: None,
        horizontal: Some(15),
        vertical: Some(25),
    };
    let domain = dto.into_domain();
    assert_eq!(domain.top().value(), 5);
    assert_eq!(domain.bottom().value(), 25);
    assert_eq!(domain.left().value(), 8);
    assert_eq!(domain.right().value(), 15);
}

#[test]
fn test_partial_margin_dto_all() {
    let dto = PartialMarginConfigDto::All(10);
    let domain = dto.into_domain();
    assert_eq!(domain.top().unwrap().value(), 10);
    assert_eq!(domain.bottom().unwrap().value(), 10);
    assert_eq!(domain.left().unwrap().value(), 10);
    assert_eq!(domain.right().unwrap().value(), 10);
}

#[test]
fn test_partial_margin_dto_fields() {
    let dto = PartialMarginConfigDto::Fields {
        top: Some(5),
        bottom: None,
        left: None,
        right: None,
        horizontal: Some(15),
        vertical: None,
    };
    let domain = dto.into_domain();
    assert_eq!(domain.top().unwrap().value(), 5);
    assert!(domain.bottom().is_none());
    assert_eq!(domain.left().unwrap().value(), 15);
    assert_eq!(domain.right().unwrap().value(), 15);
}

#[test]
fn test_json_value_to_dynamic_nested() {
    use crate::shared::primitives::DynamicValue;
    use serde_json::json;

    let v = json!({
        "null": null,
        "bool": true,
        "num": 42.5,
        "str": "hello",
        "arr": [1, 2, "three"],
        "nested": {
            "inner": "val"
        }
    });

    let dyn_val = helpers::json_value_to_dynamic(v);
    if let DynamicValue::Map(m) = dyn_val {
        assert_eq!(m.get("null"), Some(&DynamicValue::Null));
        assert_eq!(m.get("bool"), Some(&DynamicValue::Bool(true)));
        assert_eq!(m.get("num"), Some(&DynamicValue::Number(42.5)));
        assert_eq!(
            m.get("str"),
            Some(&DynamicValue::String("hello".to_string()))
        );
        if let Some(DynamicValue::Array(arr)) = m.get("arr") {
            assert_eq!(arr.len(), 3);
        } else {
            panic!("expected array");
        }
    } else {
        panic!("expected map");
    }
}

#[test]
fn test_popup_config_dto_offset_default() {
    let toml_str = r#"
        behavior = "global"
    "#;
    let dto: PopupConfigDto = toml::from_str(toml_str).unwrap();
    let domain = dto.into_domain();
    assert_eq!(domain.offset().dx(), 0);
    assert_eq!(domain.offset().dy(), 8);
}

#[test]
fn test_popup_config_dto_offset_map() {
    let toml_str = r#"
        behavior = "per_monitor"
        offset = { x = 4, y = 16 }
    "#;
    let dto: PopupConfigDto = toml::from_str(toml_str).unwrap();
    let domain = dto.into_domain();
    assert_eq!(domain.behavior(), crate::shared::config::domain::PopupBehavior::PerMonitor);
    assert_eq!(domain.offset().dx(), 4);
    assert_eq!(domain.offset().dy(), 16);
}

#[test]
fn test_popup_config_dto_offset_list() {
    let toml_str = r#"
        behavior = "global"
        offset = [2, 10]
    "#;
    let dto: PopupConfigDto = toml::from_str(toml_str).unwrap();
    let domain = dto.into_domain();
    assert_eq!(domain.offset().dx(), 2);
    assert_eq!(domain.offset().dy(), 10);
}
