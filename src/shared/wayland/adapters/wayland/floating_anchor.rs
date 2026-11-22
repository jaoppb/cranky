use super::floating_bounds::{AnchorExtent, compute_module_bounds, to_i32};
use super::floating_teardown::{TeardownCause, teardown_floating};
use super::state::WaylandState;
use super::tooltip_anchor::resolve_tooltip_anchor;
use super::types::ModuleSurface;
use crate::features::layout_engine::domain::{
    FloatingKind, OpeningPopup, PopupExclusivity, PopupTarget, ShowFloating,
};
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
pub(super) enum FloatingParentRole {
    Layer(ZwlrLayerSurfaceV1),
    Popup(XdgSurface),
}

pub(super) struct AnchorInfo {
    pub(crate) bar_scale: i32,
    pub(crate) parent_role: FloatingParentRole,
    pub(crate) anchor_x: i32,
    pub(crate) anchor_y: i32,
    pub(crate) anchor_w: i32,
    pub(crate) anchor_h: i32,
    pub(crate) target_monitor_id: MonitorId,
}

pub(super) fn handle_conflicts(
    state: &mut WaylandState,
    target: &PopupTarget,
    ancestor_kinds: &[FloatingKind],
) {
    let behavior = state.hub.config_rx().borrow().popup().behavior();
    let active_targets: Vec<PopupTarget> = state
        .floating_surfaces
        .keys()
        .filter_map(|kind| match kind {
            FloatingKind::Popup(popup_target) => Some(popup_target.clone()),
            _ => None,
        })
        .collect();
    let ancestor_targets: Vec<PopupTarget> = ancestor_kinds
        .iter()
        .filter_map(|kind| match kind {
            FloatingKind::Popup(popup_target) => Some(popup_target.clone()),
            _ => None,
        })
        .collect();
    let conflicts = PopupExclusivity::compute_conflicts(
        &active_targets,
        OpeningPopup::new(target, &ancestor_targets),
        behavior,
    );
    for conf in conflicts {
        teardown_floating(state, &FloatingKind::Popup(conf), TeardownCause::External);
    }
}

/// Where `request`'s surface anchors and what it is parented to, or `None`
/// while its owner's bar or parent surface does not exist.
pub(super) fn resolve_anchor(state: &WaylandState, request: &ShowFloating) -> Option<AnchorInfo> {
    let (kind, parent) = (&request.kind, request.parent.as_ref());
    let mut bar_scale = 1;
    let mut parent_role = None;
    let mut anchor_x = 0;
    let mut anchor_y = 0;
    let mut anchor_w = 1;
    let mut anchor_h = 1;
    let mut target_monitor_id = request
        .monitor_id
        .clone()
        .unwrap_or_else(|| MonitorId::new(""));

    match kind {
        FloatingKind::Tooltip => {
            let resolved = resolve_tooltip_anchor(state)?;
            bar_scale = resolved.bar_scale;
            parent_role = Some(resolved.parent_role);
            target_monitor_id = resolved.target_monitor_id;
            anchor_x = resolved.anchor_x;
            anchor_y = resolved.anchor_y;
        }
        FloatingKind::Popup(target) | FloatingKind::Panel(target) => {
            let is_panel = matches!(kind, FloatingKind::Panel(_));
            let extent = if is_panel || parent.is_some() {
                AnchorExtent::Exact
            } else {
                AnchorExtent::BarColumn
            };
            let module_id = target.module_id();
            let mon_name = target.monitor_id().as_str();
            for bar in &state.bars {
                if bar.output_name == mon_name {
                    bar_scale = bar.scale;
                    target_monitor_id = target.monitor_id().clone();

                    parent_role = parent.map_or_else(
                        || Some(FloatingParentRole::Layer(bar.layer_surface.clone())),
                        |parent_kind| {
                            state.floating_surfaces.get(parent_kind).map(|floating| {
                                FloatingParentRole::Popup(floating.xdg_surface.clone())
                            })
                        },
                    );
                    let placement = parent.map_or_else(
                        || {
                            bar.module_surfaces
                                .get(&module_id)
                                .map(ModuleSurface::placement)
                        },
                        |parent_kind| {
                            state
                                .floating_surfaces
                                .get(parent_kind)
                                .and_then(|floating| floating.child_placement(module_id))
                        },
                    );
                    let bar_height = to_i32(bar.height);
                    let bounds = compute_module_bounds(placement, request.anchor_rect, bar_height)
                        .spanning(extent, bar_height);
                    anchor_x = bounds.x;
                    anchor_y = bounds.y;
                    anchor_w = bounds.width;
                    anchor_h = bounds.height;
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
