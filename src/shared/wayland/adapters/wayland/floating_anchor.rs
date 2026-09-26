use super::floating_bounds::{compute_module_bounds, to_i32};
use super::floating_teardown::teardown_floating;
use super::state::WaylandState;
use crate::features::layout_engine::domain::{FloatingKind, PopupExclusivity, PopupTarget};
use crate::shared::primitives::geometry::Rect;
use crate::shared::primitives::MonitorId;
use wayland_protocols::xdg::shell::client::xdg_surface::XdgSurface;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::ZwlrLayerSurfaceV1;

/// What a new `xdg_popup` gets parented to: the bar's own layer surface for
/// one owned by a module embedded directly in the bar tree, or another
/// floating surface's own `xdg_surface` for one nested inside it (decision
/// 7). Only the top-level case calls `zwlr_layer_surface_v1.get_popup` —
/// wlr-layer-shell only associates the popup that is the layer surface's
/// direct child that way; a nested popup names its parent via
/// `xdg_surface.get_popup(Some(parent), ...)` instead.
pub(crate) enum FloatingParentRole {
    Layer(ZwlrLayerSurfaceV1),
    Popup(XdgSurface),
}

pub(crate) struct AnchorInfo {
    pub bar_scale: i32,
    pub parent_role: FloatingParentRole,
    pub anchor_x: i32,
    pub anchor_y: i32,
    pub anchor_w: i32,
    pub anchor_h: i32,
    pub target_monitor_id: MonitorId,
}

pub(crate) fn handle_conflicts(
    state: &mut WaylandState,
    target: &PopupTarget,
    ancestor_kinds: &[FloatingKind],
) {
    let behavior = state.hub.config_rx().borrow().popup().behavior();
    let active_targets: Vec<PopupTarget> = state
        .floating_surfaces
        .keys()
        .filter_map(|k| match k {
            FloatingKind::Popup(t) => Some(t.clone()),
            _ => None,
        })
        .collect();
    let ancestor_targets: Vec<PopupTarget> = ancestor_kinds
        .iter()
        .filter_map(|k| match k {
            FloatingKind::Popup(t) => Some(t.clone()),
            _ => None,
        })
        .collect();
    let conflicts =
        PopupExclusivity::compute_conflicts(&active_targets, target, &ancestor_targets, behavior);
    for conf in conflicts {
        teardown_floating(state, &FloatingKind::Popup(conf), true);
    }
}

pub(crate) fn resolve_anchor(
    state: &WaylandState,
    kind: &FloatingKind,
    monitor_id: Option<MonitorId>,
    anchor_rect: Option<Rect>,
    parent: Option<&FloatingKind>,
) -> Option<AnchorInfo> {
    let mut bar_scale = 1;
    let mut bar_height = 0;
    let mut parent_role = None;
    let mut anchor_x = 0;
    let mut anchor_y = 0;
    let mut anchor_w = 1;
    let mut anchor_h = 1;
    let mut target_monitor_id = monitor_id.unwrap_or_else(|| MonitorId::new(""));

    match kind {
        FloatingKind::Tooltip => {
            let parent_surface = state.pointer_surface.clone()?;
            let mut pointer_x = state.pointer_pos.0;
            for bar in &state.bars {
                if bar.surface == parent_surface {
                    bar_scale = bar.scale;
                    bar_height = bar.height;
                    parent_role = Some(FloatingParentRole::Layer(bar.layer_surface.clone()));
                    target_monitor_id = MonitorId::new(&bar.output_name);
                    break;
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
                    break;
                }
            }
            anchor_x = i32::try_from(crate::utils::f64_to_i64(pointer_x)).unwrap_or(0);
            anchor_y = to_i32(bar_height);
        }
        FloatingKind::Popup(target) | FloatingKind::Panel(target) => {
            let is_panel = matches!(kind, FloatingKind::Panel(_));
            let precise = is_panel || parent.is_some();
            let module_id = target.module_id();
            let mon_name = target.monitor_id().as_str();
            for bar in &state.bars {
                if bar.output_name == mon_name {
                    bar_scale = bar.scale;
                    target_monitor_id = target.monitor_id().clone();

                    parent_role = parent.map_or_else(
                        || Some(FloatingParentRole::Layer(bar.layer_surface.clone())),
                        |parent_kind| {
                            state
                                .floating_surfaces
                                .get(parent_kind)
                                .map(|f| FloatingParentRole::Popup(f.xdg_surface.clone()))
                        },
                    );
                    let ms = parent.map_or_else(
                        || bar.module_surfaces.get(&module_id),
                        |parent_kind| {
                            state
                                .floating_surfaces
                                .get(parent_kind)
                                .and_then(|f| f.module_surfaces.get(&module_id))
                        },
                    );
                    let bounds =
                        compute_module_bounds(ms, anchor_rect, to_i32(bar.height), precise);
                    anchor_x = bounds.x;
                    anchor_y = bounds.y;
                    anchor_w = bounds.w;
                    anchor_h = bounds.h;
                    break;
                }
            }
        }
    }

    let parent_role = parent_role?;
    Some(AnchorInfo {
        bar_scale,
        parent_role,
        anchor_x,
        anchor_y,
        anchor_w,
        anchor_h,
        target_monitor_id,
    })
}
