//! How the read model lays out the bar's slots.

use crate::shared::config::domain::{
    Config, CreatePartialRootConfigCommand, PartialRootConfig, RootConfig,
};
use crate::shared::primitives::geometry::{BarHeight, BarWidth, Size};
use crate::shared::primitives::{DynamicValue, ModuleId, ModuleName, ModuleOptions, MonitorId};
use crate::shared::wayland::domain::AppReadModel;
use crate::shared::wayland::ports::MonitorBar;
use std::collections::HashMap;

/// Root options placing, in each slot, the modules named in it.
fn slots(entries: &[(&str, &[&str])]) -> ModuleOptions {
    let names = |names: &[&str]| {
        DynamicValue::Array(
            names
                .iter()
                .map(|name| DynamicValue::String((*name).to_string()))
                .collect(),
        )
    };
    ModuleOptions::new(
        entries
            .iter()
            .map(|(slot, slot_names)| ((*slot).to_string(), names(slot_names)))
            .collect(),
    )
}

/// A read model of `config` holding the modules `(name, id, size)`, sized on `DP-1`.
fn read_model_with(config: Config, modules: &[(&str, u32, Size)]) -> AppReadModel {
    let ids = modules.iter().map(|(_, id, _)| ModuleId::new(*id));
    let named = |(name, id, _): &(&str, u32, Size)| (ModuleName::new(*name), ModuleId::new(*id));
    let sizes = modules
        .iter()
        .map(|(_, id, size)| (ModuleId::new(*id), *size));
    AppReadModel {
        config,
        root_module: None,
        module_ids: ids.collect(),
        module_names: modules
            .iter()
            .map(named)
            .map(|(name, id)| (id, name))
            .collect(),
        name_to_ids: modules
            .iter()
            .map(named)
            .map(|(name, id)| (name, vec![id]))
            .collect(),
        module_keys: HashMap::new(),
        module_sizes: HashMap::from([(MonitorId::new("DP-1"), sizes.collect())]),
        computed_layouts: HashMap::new(),
    }
}

#[test]
fn test_calculate_layout_unfocused() {
    let unfocused = PartialRootConfig::new(CreatePartialRootConfigCommand::new(
        Some(BarHeight::new(20)),
        None,
        None,
    ));
    let root_config = RootConfig::default()
        .with_unfocused(Some(unfocused))
        .with_options(slots(&[("center", &["clock"])]));
    let config = Config::default().with_root(root_config);
    let read_model = read_model_with(config.clone(), &[("clock", 1, Size::new(50, 10))]);
    let bar = MonitorBar::new(MonitorId::new("DP-1"), BarWidth::new(1920));

    // 1. Calculate with focused config
    let layouts_focused = read_model.calculate_layout(&bar, config.root());
    assert_eq!(layouts_focused.len(), 1);
    // height 30, available height = 30, module height = 10, y should be (30 - 10) / 2 = 10
    assert_eq!(layouts_focused[0].bounds().position().y(), 10);

    // 2. Calculate with unfocused config
    let layouts_unfocused = read_model.calculate_layout(&bar, &config.root().as_unfocused());
    assert_eq!(layouts_unfocused.len(), 1);
    // height 20, available height = 20, module height = 10, y should be (20 - 10) / 2 = 5
    assert_eq!(layouts_unfocused[0].bounds().position().y(), 5);
}

#[test]
fn test_calculate_layout_left_right() {
    let options = slots(&[("left", &["m1", "m2"]), ("right", &["m3"])]);
    let config = Config::default().with_root(RootConfig::default().with_options(options));
    let read_model = read_model_with(
        config.clone(),
        &[
            ("m1", 1, Size::new(100, 20)),
            ("m2", 2, Size::new(50, 20)),
            ("m3", 3, Size::new(80, 20)),
        ],
    );

    let bar = MonitorBar::new(MonitorId::new("DP-1"), BarWidth::new(1920));
    let layouts = read_model.calculate_layout(&bar, config.root());
    assert_eq!(layouts.len(), 3);

    let gap = 8;
    let padding_h = 8;
    let x_of = |id: u32| {
        let layout = layouts
            .iter()
            .find(|layout| layout.id() == ModuleId::new(id));
        layout.unwrap().bounds().x()
    };
    assert_eq!(x_of(1), padding_h);
    assert_eq!(x_of(2), padding_h + 100 + gap);
    assert_eq!(x_of(3), 1920 - padding_h - 80);
}
