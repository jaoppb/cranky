use super::floating_anchor::FloatingParentRole;
use super::floating_bounds::to_i32;
use super::state::WaylandState;
use crate::features::layout_engine::domain::FloatingKind;
use crate::shared::primitives::MonitorId;

/// What `resolve_tooltip_anchor` found: everything `resolve_anchor` needs
/// from the `Tooltip` arm, computed in one place so that arm stays a
/// handful of lines regardless of how many surface kinds it has to search.
pub(super) struct TooltipAnchor {
    pub(super) bar_scale: i32,
    pub(super) parent_role: FloatingParentRole,
    pub(super) target_monitor_id: MonitorId,
    pub(super) anchor_x: i32,
    pub(super) anchor_y: i32,
}

/// Finds the bar or floating surface that owns the hovered element and
/// derives a tooltip's anchor from it. Tries every bar first (a module
/// embedded directly in the bar tree, or the bar itself), then every
/// currently open floating surface's own `module_surfaces` — structurally
/// parallel to a bar's — for a module embedded inside an open popup/panel
/// (`ui.module()` nested inside floating content). Without the second
/// pass, hovering such a module would leave the tooltip anchor unresolved.
pub(super) fn resolve_tooltip_anchor(state: &WaylandState) -> Option<TooltipAnchor> {
    let parent_surface = state.pointer_surface.clone()?;
    let mut pointer_x = state.pointer_pos.0;
    let mut bar_scale = 1;
    let mut bar_height = 0;
    let mut parent_role = None;
    let mut target_monitor_id = MonitorId::new("");

    'search: {
        for bar in &state.bars {
            if bar.surface == parent_surface {
                bar_scale = bar.scale;
                bar_height = bar.height;
                parent_role = Some(FloatingParentRole::Layer(bar.layer_surface.clone()));
                target_monitor_id = MonitorId::new(&bar.output_name);
                break 'search;
            }
            if let Some(ms) = bar
                .module_surfaces
                .values()
                .find(|m| m.surface == parent_surface)
            {
                bar_scale = bar.scale;
                bar_height = bar.height;
                parent_role = Some(FloatingParentRole::Layer(bar.layer_surface.clone()));
                pointer_x += f64::from(ms.x);
                target_monitor_id = MonitorId::new(&bar.output_name);
                break 'search;
            }
        }
        for (floating_kind, floating) in &state.floating_surfaces {
            let Some(floating_monitor_id) = (match floating_kind {
                FloatingKind::Popup(t) | FloatingKind::Panel(t) => Some(t.monitor_id()),
                FloatingKind::Tooltip => None,
            }) else {
                continue;
            };
            let Some(ms) = floating
                .module_surfaces
                .values()
                .find(|m| m.surface == parent_surface)
            else {
                continue;
            };
            if let Some(bar) = state
                .bars
                .iter()
                .find(|bar| bar.output_name == floating_monitor_id.as_str())
            {
                bar_scale = bar.scale;
                bar_height = bar.height;
            }
            parent_role = Some(FloatingParentRole::Popup(floating.xdg_surface.clone()));
            pointer_x += f64::from(ms.x);
            target_monitor_id = floating_monitor_id.clone();
            break 'search;
        }
    }

    Some(TooltipAnchor {
        bar_scale,
        parent_role: parent_role?,
        target_monitor_id,
        anchor_x: i32::try_from(crate::utils::f64_to_i64(pointer_x)).unwrap_or(0),
        anchor_y: to_i32(bar_height),
    })
}
