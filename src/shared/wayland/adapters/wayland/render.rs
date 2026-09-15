use super::state::WaylandState;
use crate::features::module_runtime::ports::LayoutSender;
use crate::shared::primitives::geometry::{Position, Rect, Size};
use crate::shared::primitives::{ChildBounds, ModuleId, MonitorId, SizeConstraint};
use crate::shared::wayland::ports::AppReadModel;
use std::collections::HashMap;
use tracing::{debug, info_span};
use wayland_client::Connection;

pub(crate) fn render_outputs(
    state: &mut WaylandState,
    connection: &Connection,
    read_model: &AppReadModel,
    layout_senders: &HashMap<ModuleId, Box<dyn LayoutSender>>,
) {
    let span = info_span!("render_all_outputs");
    let _enter = span.enter();

    if state.bars.is_empty() {
        debug!("No bars available for rendering.");
        return;
    }

    let mut all_layouts_by_module: HashMap<ModuleId, HashMap<MonitorId, ChildBounds>> =
        HashMap::new();

    for bar in &mut state.bars {
        if !bar.configured {
            debug!("Skipping render for unconfigured bar: {}", bar.output_name);
            continue;
        }
        let width = bar.width;

        let is_focused = state
            .hub
            .hyprland_rx()
            .borrow()
            .focused_monitor()
            .is_some_and(|name| name.as_str() == bar.output_name);

        let mut root_config = read_model.config().root().clone();
        if !is_focused {
            root_config = root_config.as_unfocused();
        }

        // Check if hot-reload of height or margin is needed
        if bar.config_height != root_config.height().value()
            || bar.config_margin != *root_config.margin()
        {
            debug!(
                "Hot-reloading bar height/margin for output: {}",
                bar.output_name
            );
            bar.config_height = root_config.height().value();
            bar.config_margin = *root_config.margin();

            let margin = root_config.margin();
            bar.layer_surface.set_size(0, bar.config_height);
            bar.layer_surface.set_margin(
                margin.top().value(),
                margin.right().value(),
                margin.bottom().value(),
                margin.left().value(),
            );
            let height_i32 = i32::try_from(bar.config_height).unwrap_or(0);
            let zone = height_i32
                .saturating_add(margin.top().value())
                .saturating_add(margin.bottom().value());
            bar.layer_surface.set_exclusive_zone(zone);
            bar.surface.commit();
        }

        let monitor_id = MonitorId::new(&bar.output_name);
        if let Some(root_id) = read_model.root_module() {
            let bar_rect = Rect::new(
                Position::new(0, 0),
                Size::new(width, bar.config_height),
            );
            // The root always gets a full, two-axis pin matching its own
            // layer-surface bounds — it has no parent to leave an axis free
            // for, so this reproduces exactly the sizing it always had.
            let constraint = SizeConstraint::new(Some(width), Some(bar.config_height));
            all_layouts_by_module
                .entry(root_id)
                .or_default()
                .insert(monitor_id, ChildBounds::new(bar_rect, constraint));
        }
    }

    // Broadcast root layout bounds to the root module for ALL active monitors
    for (id, sender) in layout_senders {
        if let Some(rects) = all_layouts_by_module.get(id) {
            sender.send_layout(rects.clone());
        }
    }

    let _ = connection.flush();
}
