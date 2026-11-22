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
        let bar_height = root_config.height().value();
        let bar_width = bar.width.value();
        let root_size = Size::new(bar_width, bar_height);
        let mut layouts: Vec<ModuleLayout> = self.root_layout(root_size).into_iter().collect();
        if let Some(computed) = self.computed_layouts_on(&bar.monitor) {
            layouts.extend(computed);
            return layouts;
        }

        let row = SlotRow {
            model: self,
            monitor: &bar.monitor,
            height: i32::try_from(bar_height).unwrap_or(0),
            width: i32::try_from(bar_width).unwrap_or(0),
        };
        layouts.extend(row.pack_left(self.slot_module_ids(root_config, "left")));
        layouts.extend(row.pack_right(self.slot_module_ids(root_config, "right")));
        layouts.extend(row.pack_center(self.slot_module_ids(root_config, "center")));
        layouts
    }

    /// The root module over the whole `size` bar, if there is a root.
    #[cfg(test)]
    fn root_layout(&self, size: Size) -> Option<ModuleLayout> {
        self.root_module.map(|id| ModuleLayout {
            id,
            bounds: Rect::new(Position::new(0, 0), size),
        })
    }

    /// Every module but the root laid out on `monitor`, if layouts were
    /// computed for it.
    #[cfg(test)]
    fn computed_layouts_on(&self, monitor: &MonitorId) -> Option<Vec<ModuleLayout>> {
        let mon_layouts = self.computed_layouts.get(monitor)?;
        let layouts = mon_layouts
            .iter()
            .filter(|&(&id, _)| Some(id) != self.root_module)
            .map(|(&id, &bounds)| ModuleLayout {
                id,
                bounds: bounds.rect(),
            });
        Some(layouts.collect())
    }

    /// The modules named in the root option `key`, in order, each name
    /// standing for every module instance it has.
    #[cfg(test)]
    fn slot_module_ids(&self, root_config: &RootConfig, key: &str) -> Vec<ModuleId> {
        let Some(DynamicValue::Array(values)) = root_config.options().get(key) else {
            return Vec::new();
        };
        values
            .iter()
            .filter_map(DynamicValue::as_str)
            .filter_map(|name| self.name_to_ids.get(&ModuleName::new(name)))
            .flatten()
            .copied()
            .collect()
    }
}

/// One bar's row for the test layout: modules packed from the left, from
/// the right and in the centre, each centred vertically.
#[cfg(test)]
struct SlotRow<'a> {
    model: &'a AppReadModel,
    monitor: &'a MonitorId,
    height: i32,
    width: i32,
}

#[cfg(test)]
impl SlotRow<'_> {
    const GAP: i32 = 8;
    const PADDING: i32 = 8;

    /// `id`'s size on this row's monitor, zero when it reported none.
    fn size_of(&self, id: ModuleId) -> Size {
        self.model
            .module_sizes
            .get(self.monitor)
            .and_then(|sizes| sizes.get(&id))
            .copied()
            .unwrap_or(Size::new(0, 0))
    }

    /// `id` with its left edge at `x`, centred vertically, and its width.
    fn place(&self, id: ModuleId, x: i32) -> (ModuleLayout, i32) {
        let size = self.size_of(id);
        let mod_height = i32::try_from(size.height()).unwrap_or(0);
        let mod_width = i32::try_from(size.width()).unwrap_or(0);
        let y = self.height.saturating_sub(mod_height).max(0) / 2;
        let bounds = Rect::new(Position::new(x, y), size);
        (ModuleLayout { id, bounds }, mod_width)
    }

    /// `ids` side by side from `x`, a gap apart.
    fn pack_from(&self, ids: Vec<ModuleId>, x: i32) -> Vec<ModuleLayout> {
        let mut next_x = x;
        ids.into_iter()
            .map(|id| {
                let (layout, mod_width) = self.place(id, next_x);
                next_x = next_x.saturating_add(mod_width).saturating_add(Self::GAP);
                layout
            })
            .collect()
    }

    fn pack_left(&self, ids: Vec<ModuleId>) -> Vec<ModuleLayout> {
        self.pack_from(ids, Self::PADDING)
    }

    /// `ids` side by side, the last one against the right padding.
    fn pack_right(&self, ids: Vec<ModuleId>) -> Vec<ModuleLayout> {
        let mut right_x = self.width.saturating_sub(Self::PADDING);
        let mut layouts: Vec<ModuleLayout> = ids
            .into_iter()
            .rev()
            .map(|id| {
                let width = i32::try_from(self.size_of(id).width()).unwrap_or(0);
                right_x = right_x.saturating_sub(width);
                let (layout, _) = self.place(id, right_x);
                right_x = right_x.saturating_sub(Self::GAP);
                layout
            })
            .collect();
        layouts.reverse();
        layouts
    }

    /// `ids` side by side, centred on the bar as one block.
    fn pack_center(&self, ids: Vec<ModuleId>) -> Vec<ModuleLayout> {
        let widths = ids
            .iter()
            .map(|&id| i32::try_from(self.size_of(id).width()).unwrap_or(0))
            .fold(0_i32, i32::saturating_add);
        let gap_count = i32::try_from(ids.len().saturating_sub(1)).unwrap_or(0);
        let center_width = widths.saturating_add(gap_count.saturating_mul(Self::GAP));
        self.pack_from(ids, (self.width.saturating_sub(center_width)) / 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let root = RootConfig::default()
            .with_height(BarHeight::new(BAR_HEIGHT))
            .with_options(ModuleOptions::new(options));
        let config = Config::default().with_root(root);
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
