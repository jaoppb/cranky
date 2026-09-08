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
    let popup = PopupConfig::new(PopupBehavior::PerMonitor, crate::shared::primitives::PopupOffset::new(0, 8));
    assert_eq!(popup.behavior(), PopupBehavior::PerMonitor);
    assert_eq!(popup.offset(), crate::shared::primitives::PopupOffset::new(0, 8));
    assert_eq!(PopupBehavior::default(), PopupBehavior::Global);
    assert_eq!(PopupConfig::default().offset(), crate::shared::primitives::PopupOffset::new(0, 8));
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

#[test]
fn test_tooltip_config_builder() {
    let bg = crate::shared::primitives::DrawingColor::Solid(crate::shared::primitives::Color::new(10, 20, 30, 255));
    let bc = crate::shared::primitives::DrawingColor::Solid(crate::shared::primitives::Color::new(40, 50, 60, 255));
    let tc = crate::shared::primitives::DrawingColor::Solid(crate::shared::primitives::Color::new(70, 80, 90, 255));
    let font = Some(FontFamily::new("Roboto".to_string()));
    let size = Some(FontSize::new(14.0));
    let radius = BorderRadius::new(6.0);
    let border_width = BorderSize::new(2.0);
    let padding = PaddingOffset::new(12);

    let config = TooltipConfig::default()
        .with_background(bg.clone())
        .with_border_color(bc.clone())
        .with_text_color(tc.clone())
        .with_font(font.clone())
        .with_size(size)
        .with_radius(radius)
        .with_border_width(border_width)
        .with_padding(padding);

    assert_eq!(config.background(), &bg);
    assert_eq!(config.border_color(), &bc);
    assert_eq!(config.text_color(), &tc);
    assert_eq!(config.font(), font.as_ref());
    assert_eq!(config.size(), size);
    assert_eq!(config.radius(), radius);
    assert_eq!(config.border_width(), border_width);
    assert_eq!(config.padding(), padding);
}
