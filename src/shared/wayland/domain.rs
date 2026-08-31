use crate::shared::config::domain::{Config, RootConfig};
use crate::shared::primitives::{
    DynamicValue, ModuleId, ModuleName, MonitorId,
    geometry::{BarWidth, Position, Rect, Size},
};
use std::collections::HashMap;

pub struct ModuleLayout {
    id: ModuleId,
    bounds: Rect,
}

impl ModuleLayout {
    #[must_use]
    pub const fn new(id: ModuleId, bounds: Rect) -> Self {
        Self { id, bounds }
    }

    #[must_use]
    pub const fn id(&self) -> ModuleId {
        self.id
    }

    #[must_use]
    pub const fn bounds(&self) -> &Rect {
        &self.bounds
    }
}

pub struct AppReadModel {
    pub config: Config,
    pub root_module: Option<ModuleId>,
    pub module_ids: Vec<ModuleId>,
    pub module_names: HashMap<ModuleId, ModuleName>,
    pub name_to_ids: HashMap<ModuleName, Vec<ModuleId>>,
    pub module_sizes: HashMap<MonitorId, HashMap<ModuleId, Size>>,
    pub computed_layouts: HashMap<MonitorId, HashMap<ModuleId, Rect>>,
}

impl AppReadModel {
    #[must_use]
    pub fn new(
        config: Config,
        root_module: Option<ModuleId>,
        module_ids: Vec<ModuleId>,
        module_names: HashMap<ModuleId, ModuleName>,
        name_to_ids: HashMap<ModuleName, Vec<ModuleId>>,
    ) -> Self {
        Self {
            config,
            root_module,
            module_ids,
            module_names,
            name_to_ids,
            module_sizes: HashMap::new(),
            computed_layouts: HashMap::new(),
        }
    }

    #[must_use]
    pub const fn config(&self) -> &Config {
        &self.config
    }

    #[must_use]
    pub const fn root_module(&self) -> Option<ModuleId> {
        self.root_module
    }

    #[must_use]
    pub fn calculate_layout(
        &self,
        monitor: &MonitorId,
        bar_width: BarWidth,
        root_config: &RootConfig,
    ) -> Vec<ModuleLayout> {
        let mut layouts = Vec::new();
        let bar_height = root_config.height().value();
        let bar_width_val = bar_width.value();

        if let Some(root_id) = self.root_module {
            layouts.push(ModuleLayout {
                id: root_id,
                bounds: Rect::new(Position::new(0, 0), Size::new(bar_width_val, bar_height)),
            });
        }

        if let Some(mon_layouts) = self.computed_layouts.get(monitor) {
            for (&mod_id, &bounds) in mon_layouts {
                if Some(mod_id) != self.root_module {
                    layouts.push(ModuleLayout { id: mod_id, bounds });
                }
            }
            return layouts;
        }

        let get_size = |id: &ModuleId| {
            self.module_sizes
                .get(monitor)
                .and_then(|m| m.get(id))
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
                for v in arr {
                    if let Some(name_str) = v.as_str() {
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
