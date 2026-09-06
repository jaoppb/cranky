use std::collections::HashMap;

use crate::shared::primitives::ModuleName;
use crate::shared::primitives::ModuleOptions;

use super::*;

#[test]
fn test_font_family() {
    let f = FontFamily::new("Inter".into());
    assert_eq!(f.as_str(), "Inter");
}

#[test]
fn test_font_size() {
    let s = FontSize::new(12.5);
    assert!((s.value() - 12.5).abs() < f32::EPSILON);
}

#[test]
fn test_margin_offset() {
    let m = MarginOffset::new(15);
    assert_eq!(m.value(), 15);
}

#[test]
fn test_margin_config() {
    let m = MarginConfig::new(
        MarginOffset::new(1),
        MarginOffset::new(2),
        MarginOffset::new(3),
        MarginOffset::new(4),
    );
    assert_eq!(m.top().value(), 1);
    assert_eq!(m.bottom().value(), 2);
    assert_eq!(m.left().value(), 3);
    assert_eq!(m.right().value(), 4);
}

#[test]
fn test_root_config_defaults() {
    let root = RootConfig::default();
    assert_eq!(root.name().as_str(), "bar");
    assert_eq!(root.height().value(), 30);
    assert_eq!(root.vertical_alignment(), VerticalAlignment::Center);

    let unfocused = root.as_unfocused();
    assert_eq!(unfocused.height().value(), 30);
}

#[test]
fn test_modules_config() {
    let mut modules_map = HashMap::new();
    modules_map.insert(
        ModuleName::new("time"),
        ModuleConfig::new(
            ModuleName::new("time"),
            true,
            EngineSelection::Auto,
            ModuleOptions::default(),
        ),
    );
    let modules = ModulesConfig::new(modules_map);

    let config = Config::new(
        RootConfig::default(),
        modules,
        RenderingMode::default(),
        crate::features::metrics::domain::MetricsConfig::default(),
        TooltipConfig::default(),
        PopupConfig::default(),
    );
    assert!(config.modules().get(&ModuleName::new("time")).is_some());
    assert_eq!(
        config
            .modules()
            .get(&ModuleName::new("time"))
            .unwrap()
            .name(),
        "time"
    );
    assert_eq!(config.popup().behavior(), PopupBehavior::Global);
}

#[test]
fn test_popup_config() {
    let popup = PopupConfig::new(PopupBehavior::PerMonitor);
    assert_eq!(popup.behavior(), PopupBehavior::PerMonitor);
    assert_eq!(PopupBehavior::default(), PopupBehavior::Global);
}

#[test]
fn test_module_config_engine() {
    let explicit = EngineSelection::Explicit(EngineId::new("rhai"));
    let cfg = ModuleConfig::new(
        "clock".into(),
        true,
        explicit.clone(),
        ModuleOptions::default(),
    );
    assert_eq!(cfg.engine(), &explicit);
    assert_eq!(
        cfg.engine().as_explicit().map(EngineId::as_str),
        Some("rhai")
    );
    assert!(!cfg.engine().is_auto());
    assert_eq!(cfg.name(), "clock");
    assert!(cfg.is_enabled());
    assert!(cfg.options().is_empty());

    let auto = EngineSelection::default();
    assert!(auto.is_auto());
    assert_eq!(auto.as_explicit(), None);
}
