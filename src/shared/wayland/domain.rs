use crate::shared::config::domain::Config;
#[cfg(test)]
use crate::shared::config::domain::RootConfig;
#[cfg(test)]
use crate::shared::primitives::DynamicValue;
#[cfg(test)]
use crate::shared::primitives::geometry::{BarWidth, Position, Rect};
use crate::shared::primitives::{
    ChildBounds, ModuleId, ModuleKey, ModuleName, MonitorId, geometry::Size,
};
use std::collections::HashMap;

/// A module's id with its laid-out bounds; only tests build it.
#[cfg(test)]
#[derive(Debug)]
pub struct ModuleLayout {
    id: ModuleId,
    bounds: Rect,
}

#[cfg(test)]
impl ModuleLayout {
    #[must_use]
    pub(crate) const fn id(&self) -> ModuleId {
        self.id
    }

    #[must_use]
    pub(crate) const fn bounds(&self) -> &Rect {
        &self.bounds
    }
}

/// The bar a test lays out modules on: the monitor it is on and its width.
#[cfg(test)]
#[derive(Debug)]
pub struct MonitorBar {
    monitor: MonitorId,
    width: BarWidth,
}

#[cfg(test)]
impl MonitorBar {
    /// The bar on `monitor`, `width` logical pixels wide.
    #[must_use]
    pub(crate) const fn new(monitor: MonitorId, width: BarWidth) -> Self {
        Self { monitor, width }
    }
}

/// The snapshot of app state the display server renders from: the config, the modules and their
/// names, and each monitor's module sizes and layouts.
#[derive(Debug)]
pub struct AppReadModel {
    pub(crate) config: Config,
    pub(crate) root_module: Option<ModuleId>,
    pub(crate) module_ids: Vec<ModuleId>,
    pub(crate) module_names: HashMap<ModuleId, ModuleName>,
    pub(crate) name_to_ids: HashMap<ModuleName, Vec<ModuleId>>,
    /// This module's full `(name, instance_id)` identity, mirroring
    /// `ModuleRegistry::module_keys` — needed wherever a lookup must
    /// distinguish two same-named embedding sites.
    pub(crate) module_keys: HashMap<ModuleId, ModuleKey>,
    pub(crate) module_sizes: HashMap<MonitorId, HashMap<ModuleId, Size>>,
    pub(crate) computed_layouts: HashMap<MonitorId, HashMap<ModuleId, ChildBounds>>,
}

impl AppReadModel {
    #[cfg(test)]
    #[must_use]
    /// A read model of `config` with `root_module` and no other module.
    pub(crate) fn new(config: Config, root_module: Option<ModuleId>) -> Self {
        Self {
            config,
            root_module,
            module_ids: Vec::new(),
            module_names: HashMap::new(),
            name_to_ids: HashMap::new(),
            module_keys: HashMap::new(),
            module_sizes: HashMap::new(),
            computed_layouts: HashMap::new(),
        }
    }

    #[must_use]
    pub(crate) const fn config(&self) -> &Config {
        &self.config
    }

    #[must_use]
    pub(crate) const fn root_module(&self) -> Option<ModuleId> {
        self.root_module
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn calculate_layout(
        &self,
        bar: &MonitorBar,
        root_config: &RootConfig,
    ) -> Vec<ModuleLayout> {
        let monitor = &bar.monitor;
        let mut layouts = Vec::new();
        let bar_height = root_config.height().value();
        let bar_width_val = bar.width.value();

        if let Some(root_id) = self.root_module {
            layouts.push(ModuleLayout {
                id: root_id,
                bounds: Rect::new(Position::new(0, 0), Size::new(bar_width_val, bar_height)),
            });
        }

        if let Some(mon_layouts) = self.computed_layouts.get(monitor) {
            for (&mod_id, &bounds) in mon_layouts {
                if Some(mod_id) != self.root_module {
                    layouts.push(ModuleLayout {
                        id: mod_id,
                        bounds: bounds.rect(),
                    });
                }
            }
            return layouts;
        }

        let get_size = |id: &ModuleId| {
            self.module_sizes
                .get(monitor)
                .and_then(|sizes| sizes.get(id))
                .copied()
                .unwrap_or(Size::new(0, 0))
        };

        let gap = 8_i32;
        let padding_h = 8_i32;
        let available_height_i32 = i32::try_from(bar_height).unwrap_or(0);
        let bar_width_i32 = i32::try_from(bar_width_val).unwrap_or(0);

        let get_module_ids = |key: &str| -> Vec<ModuleId> {
            let mut ids = Vec::new();
            if let Some(DynamicValue::Array(arr)) = root_config.options().get(key) {
                for value in arr {
                    if let Some(name_str) = value.as_str() {
                        let mod_name = ModuleName::new(name_str);
                        if let Some(mod_ids) = self.name_to_ids.get(&mod_name) {
                            ids.extend(mod_ids.iter().copied());
                        }
                    }
                }
            }
            ids
        };

        let left_ids = get_module_ids("left");
        let center_ids = get_module_ids("center");
        let right_ids = get_module_ids("right");

        // Calculate left modules
        let mut left_x = padding_h;
        for id in left_ids {
            let size = get_size(&id);
            let mod_height = i32::try_from(size.height()).unwrap_or(0);
            let mod_width = i32::try_from(size.width()).unwrap_or(0);
            let y = available_height_i32.saturating_sub(mod_height).max(0) / 2;
            layouts.push(ModuleLayout {
                id,
                bounds: Rect::new(Position::new(left_x, y), size),
            });
            left_x = left_x.saturating_add(mod_width).saturating_add(gap);
        }

        // Calculate right modules
        let mut right_x = bar_width_i32.saturating_sub(padding_h);
        let mut right_layouts = Vec::new();
        for id in right_ids.into_iter().rev() {
            let size = get_size(&id);
            let mod_height = i32::try_from(size.height()).unwrap_or(0);
            let mod_width = i32::try_from(size.width()).unwrap_or(0);
            right_x = right_x.saturating_sub(mod_width);
            let y = available_height_i32.saturating_sub(mod_height).max(0) / 2;
            right_layouts.push(ModuleLayout {
                id,
                bounds: Rect::new(Position::new(right_x, y), size),
            });
            right_x = right_x.saturating_sub(gap);
        }
        layouts.extend(right_layouts.into_iter().rev());

        // Calculate center modules
        let mut center_width = 0_i32;
        let mut center_sizes = Vec::new();
        for id in center_ids {
            let size = get_size(&id);
            let mod_width = i32::try_from(size.width()).unwrap_or(0);
            center_width = center_width.saturating_add(mod_width);
            center_sizes.push((id, size));
        }
        if !center_sizes.is_empty() {
            let gap_count = i32::try_from(center_sizes.len().saturating_sub(1)).unwrap_or(0);
            center_width = center_width.saturating_add(gap_count.saturating_mul(gap));
        }

        let mut center_x = (bar_width_i32.saturating_sub(center_width)) / 2;
        for (id, size) in center_sizes {
            let mod_height = i32::try_from(size.height()).unwrap_or(0);
            let mod_width = i32::try_from(size.width()).unwrap_or(0);
            let y = available_height_i32.saturating_sub(mod_height).max(0) / 2;
            layouts.push(ModuleLayout {
                id,
                bounds: Rect::new(Position::new(center_x, y), size),
            });
            center_x = center_x.saturating_add(mod_width).saturating_add(gap);
        }

        layouts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::metrics::domain::MetricsConfig;
    use crate::shared::config::domain::{
        CreateRootConfigCommand, MarginConfig, ModulesConfig, PopupConfig, RenderingMode,
        TooltipConfig, VerticalAlignment,
    };
    use crate::shared::primitives::geometry::BarHeight;
    use crate::shared::primitives::{ModuleOptions, SizeConstraint};

    const BAR_WIDTH: u32 = 1000;
    const BAR_HEIGHT: u32 = 30;

    /// A read model whose bar declares `slots` (slot name to module names),
    /// with module `n` (1-based, in declaration order) reporting `sizes[n-1]`
    /// on `DP-1`.
    fn model(slots: &[(&str, &[&str])], sizes: &[Size]) -> AppReadModel {
        let mut options = HashMap::new();
        let mut name_to_ids = HashMap::new();
        for (slot, names) in slots {
            let values = names.iter().map(|name| DynamicValue::from(*name)).collect();
            options.insert((*slot).to_string(), DynamicValue::Array(values));
        }
        let declared = slots.iter().flat_map(|(_, names)| names.iter());
        for (id, name) in (1_u32..).zip(declared) {
            name_to_ids.insert(ModuleName::new(*name), vec![ModuleId::new(id)]);
        }
        let root = RootConfig::new(CreateRootConfigCommand::new(
            ModuleName::new("bar"),
            BarHeight::new(BAR_HEIGHT),
            VerticalAlignment::Center,
            MarginConfig::default(),
            None,
            ModuleOptions::new(options),
        ));
        let config = Config::new(
            root,
            ModulesConfig::default(),
            RenderingMode::default(),
            MetricsConfig::default(),
            TooltipConfig::default(),
            PopupConfig::default(),
        );
        let mut read_model = AppReadModel::new(config, None);
        read_model.name_to_ids = name_to_ids;
        let reported = (1_u32..)
            .zip(sizes)
            .map(|(id, size)| (ModuleId::new(id), *size))
            .collect();
        read_model
            .module_sizes
            .insert(MonitorId::new("DP-1"), reported);
        read_model
    }

    fn placed(read_model: &AppReadModel, monitor: &str) -> Vec<(ModuleId, Rect)> {
        read_model
            .calculate_layout(
                &MonitorBar::new(MonitorId::new(monitor), BarWidth::new(BAR_WIDTH)),
                read_model.config().root(),
            )
            .iter()
            .map(|layout| (layout.id(), *layout.bounds()))
            .collect()
    }

    fn rect(x: i32, y: i32, size: Size) -> Rect {
        Rect::new(Position::new(x, y), size)
    }

    #[test]
    fn center_slot_is_centred_as_one_block_with_gaps_between_modules() {
        let (first, second) = (Size::new(100, 10), Size::new(60, 20));
        let read_model = model(&[("center", &["a", "b"])], &[first, second]);

        // Block width is 100 + 8 + 60 = 168, so it starts at (1000 - 168) / 2.
        assert_eq!(
            placed(&read_model, "DP-1"),
            vec![
                (ModuleId::new(1), rect(416, 10, first)),
                (ModuleId::new(2), rect(524, 5, second)),
            ]
        );
    }

    #[test]
    fn right_slot_is_packed_against_the_right_edge_in_declaration_order() {
        let (first, second) = (Size::new(100, 30), Size::new(60, 30));
        let read_model = model(&[("right", &["a", "b"])], &[first, second]);

        assert_eq!(
            placed(&read_model, "DP-1"),
            vec![
                (ModuleId::new(1), rect(824, 0, first)),
                (ModuleId::new(2), rect(932, 0, second)),
            ]
        );
    }

    #[test]
    fn module_that_has_not_reported_a_size_is_placed_with_an_empty_box() {
        let read_model = model(&[("left", &["a", "b"])], &[Size::new(100, 10)]);

        // "b" has no report on DP-1; nothing has one on DP-2.
        assert_eq!(
            placed(&read_model, "DP-1"),
            vec![
                (ModuleId::new(1), rect(8, 10, Size::new(100, 10))),
                (ModuleId::new(2), rect(116, 15, Size::new(0, 0))),
            ]
        );
        assert_eq!(
            placed(&read_model, "DP-2"),
            vec![
                (ModuleId::new(1), rect(8, 15, Size::new(0, 0))),
                (ModuleId::new(2), rect(16, 15, Size::new(0, 0))),
            ]
        );
    }

    #[test]
    fn root_module_always_spans_the_whole_bar() {
        let mut read_model = model(&[], &[]);
        read_model.root_module = Some(ModuleId::new(9));

        assert_eq!(
            placed(&read_model, "DP-1"),
            vec![(
                ModuleId::new(9),
                rect(0, 0, Size::new(BAR_WIDTH, BAR_HEIGHT))
            )]
        );
    }

    #[test]
    fn computed_layouts_replace_the_slot_fallback_on_their_monitor_only() {
        let mut read_model = model(&[("left", &["a"])], &[Size::new(100, 10)]);
        let root = ModuleId::new(9);
        read_model.root_module = Some(root);
        let computed = rect(300, 4, Size::new(40, 12));
        let bounds = |child_rect| ChildBounds::new(child_rect, SizeConstraint::none());
        read_model.computed_layouts.insert(
            MonitorId::new("DP-1"),
            HashMap::from([
                (ModuleId::new(1), bounds(computed)),
                // The root's own entry never overrides its full-bar box.
                (root, bounds(rect(1, 1, Size::new(1, 1)))),
            ]),
        );
        let full_bar = rect(0, 0, Size::new(BAR_WIDTH, BAR_HEIGHT));

        assert_eq!(
            placed(&read_model, "DP-1"),
            vec![(root, full_bar), (ModuleId::new(1), computed)]
        );
        // DP-2 has no computed layout, so it still gets the slot fallback.
        assert_eq!(
            placed(&read_model, "DP-2"),
            vec![
                (root, full_bar),
                (ModuleId::new(1), rect(8, 15, Size::new(0, 0)))
            ]
        );
    }
}
