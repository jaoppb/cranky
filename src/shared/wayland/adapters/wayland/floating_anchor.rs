use super::floating_bounds::{AnchorExtent, ChildPlacement, compute_module_bounds, to_i32};
use super::floating_teardown::{TeardownCause, teardown_floating};
use super::state::WaylandState;
use super::tooltip_anchor::resolve_tooltip_anchor;
use super::types::{ModuleSurface, WaylandBar};
use crate::features::layout_engine::domain::{
    FloatingKind, OpeningPopup, PopupExclusivity, PopupTarget, ShowFloating,
};
use crate::shared::primitives::{ModuleId, MonitorId};
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
    match &request.kind {
        FloatingKind::Tooltip => resolve_tooltip_anchor(state),
        FloatingKind::Popup(target) | FloatingKind::Panel(target) => {
            module_anchor(state, request, target)
        }
    }
}

/// Where a popup or panel owned by `target` anchors: on its module's place
/// in the bar of `target`'s monitor, or in the floating surface it nests in.
fn module_anchor(
    state: &WaylandState,
    request: &ShowFloating,
    target: &PopupTarget,
) -> Option<AnchorInfo> {
    let monitor = target.monitor_id();
    let bar = state
        .bars
        .iter()
        .find(|bar| bar.output_name == monitor.as_str())?;
    let (parent_role, placement) = match request.parent.as_ref() {
        None => bar_parent(bar, target.module_id()),
        Some(parent_kind) => floating_parent(state, parent_kind, target.module_id())?,
    };
    let bar_height = to_i32(bar.height);
    let bounds = compute_module_bounds(placement, request.anchor_rect, bar_height)
        .spanning(anchor_extent(request), bar_height);
    Some(AnchorInfo {
        bar_scale: bar.scale,
        parent_role,
        anchor_x: bounds.x,
        anchor_y: bounds.y,
        anchor_w: bounds.width,
        anchor_h: bounds.height,
        target_monitor_id: monitor.clone(),
    })
}

/// A top-level popup's parent, `bar`'s layer surface, and where `module_id`
/// sits in the bar.
fn bar_parent(
    bar: &WaylandBar,
    module_id: ModuleId,
) -> (FloatingParentRole, Option<ChildPlacement>) {
    let placement = bar
        .module_surfaces
        .get(&module_id)
        .map(ModuleSurface::placement);
    (
        FloatingParentRole::Layer(bar.layer_surface.clone()),
        placement,
    )
}

/// A nested popup's parent, the `xdg_surface` of floating surface
/// `parent_kind`, and where `module_id` sits in it; `None` while that
/// surface does not exist.
fn floating_parent(
    state: &WaylandState,
    parent_kind: &FloatingKind,
    module_id: ModuleId,
) -> Option<(FloatingParentRole, Option<ChildPlacement>)> {
    let floating = state.floating_surfaces.get(parent_kind)?;
    let role = FloatingParentRole::Popup(floating.xdg_surface.clone());
    Some((role, floating.child_placement(module_id)))
}

/// How much of its parent `request`'s surface anchors on: the exact rect for
/// a panel or a nested popup, the bar's column for a top-level popup.
const fn anchor_extent(request: &ShowFloating) -> AnchorExtent {
    if matches!(request.kind, FloatingKind::Panel(_)) || request.parent.is_some() {
        AnchorExtent::Exact
    } else {
        AnchorExtent::BarColumn
    }
}
