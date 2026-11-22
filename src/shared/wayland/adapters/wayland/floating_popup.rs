//! The Wayland objects of a new floating surface: its surface, `xdg_surface`
//! and `xdg_popup`, the seat grab a popup takes and its pointer route.
use super::floating_anchor::FloatingParentRole;
use super::floating_frame::FloatingFrame;
use super::input_region::{is_click_through, set_empty_input_region};
use super::state::WaylandState;
use crate::features::layout_engine::domain::FloatingKind;
use crate::shared::events::core::SurfaceKind;
use crate::shared::wayland::composite::route::PointerTarget;
use wayland_client::QueueHandle;
use wayland_client::protocol::{wl_compositor::WlCompositor, wl_surface::WlSurface};
use wayland_protocols::xdg::shell::client::{
    xdg_popup::XdgPopup, xdg_positioner::XdgPositioner, xdg_surface::XdgSurface,
    xdg_wm_base::XdgWmBase,
};

/// The globals a floating surface is created from, and its queue.
pub(super) struct FloatingGlobals<'a> {
    pub(super) xdg_wm_base: &'a XdgWmBase,
    pub(super) compositor: &'a WlCompositor,
    pub(super) queue_handle: &'a QueueHandle<WaylandState>,
}

/// The protocol objects of one floating surface.
pub(super) struct PopupObjects {
    pub(super) surface: WlSurface,
    pub(super) xdg_surface: XdgSurface,
    pub(super) xdg_popup: XdgPopup,
}

impl FloatingGlobals<'_> {
    /// A surface at `frame`'s anchor scale, click-through for a tooltip,
    /// with an `xdg_popup` placed by `positioner`.
    pub(super) fn open_popup(
        &self,
        frame: &FloatingFrame,
        positioner: &XdgPositioner,
    ) -> PopupObjects {
        let anchor_info = &frame.placement.anchor;
        let surface = self.compositor.create_surface(self.queue_handle, ());
        surface.set_buffer_scale(anchor_info.bar_scale);
        if is_click_through(&frame.kind) {
            set_empty_input_region(self.compositor, &surface, self.queue_handle);
        }
        let xdg_surface = self
            .xdg_wm_base
            .get_xdg_surface(&surface, self.queue_handle, ());
        // A popup nested inside another floating surface (decision 7) names its
        // parent directly; a top-level one leaves it unset and relies on
        // `zwlr_layer_surface_v1.get_popup` instead.
        let parent_xdg_surface = match &anchor_info.parent_role {
            FloatingParentRole::Popup(parent_surface) => Some(parent_surface),
            FloatingParentRole::Layer(_) => None,
        };
        let xdg_popup =
            xdg_surface.get_popup(parent_xdg_surface, positioner, self.queue_handle, ());
        PopupObjects {
            surface,
            xdg_surface,
            xdg_popup,
        }
    }
}

/// Grabs the seat for a new popup of `kind`, with the last button serial;
/// nothing else grabs, nor anything without a seat or a serial.
pub(super) fn grab_popup(state: &WaylandState, kind: &FloatingKind, xdg_popup: &XdgPopup) {
    if let FloatingKind::Popup(_) = kind
        && let (Some(seat), Some(serial)) = (state.seat.as_ref(), state.last_button_serial)
    {
        xdg_popup.grab(seat, serial.value());
    }
}

/// Where pointer events on `frame`'s surface go: its owner on the anchor's
/// monitor, for a popup or panel; a tooltip gets none.
pub(super) fn pointer_route(frame: &FloatingFrame) -> Option<PointerTarget> {
    let monitor = frame.placement.anchor.target_monitor_id.clone();
    match &frame.kind {
        FloatingKind::Popup(target) => Some((target.module_id(), monitor, SurfaceKind::Popup)),
        FloatingKind::Panel(target) => Some((target.module_id(), monitor, SurfaceKind::Panel)),
        FloatingKind::Tooltip => None,
    }
}
