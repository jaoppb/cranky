//! Click-through surfaces.
//!
//! A tooltip never grabs the seat, so on Hyprland a click that lands on it
//! (or on a subsurface of it) is a click outside the grab chain and ends the
//! popup it belongs to. Giving those surfaces an empty input region makes the
//! compositor route the pointer to whatever lies beneath instead.

use super::state::WaylandState;
use crate::features::layout_engine::domain::FloatingKind;
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_surface::WlSurface;

/// Whether surfaces owned by `kind` must let pointer input fall through.
/// Keyed on the kind, not on "has no grab": panels are grab-less too but are
/// meant to be clicked.
pub(super) const fn is_click_through(kind: &FloatingKind) -> bool {
    matches!(kind, FloatingKind::Tooltip)
}

/// Sets an empty input region on `surface`. Double-buffered: it takes effect
/// on the surface's next commit. The region object carries no further state
/// once set, so it is destroyed right away.
pub(super) fn set_empty_input_region(
    compositor: &WlCompositor,
    surface: &WlSurface,
    qh: &QueueHandle<WaylandState>,
) {
    let region = compositor.create_region(qh, ());
    surface.set_input_region(Some(&region));
    region.destroy();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::layout_engine::domain::PopupTarget;
    use crate::shared::primitives::{ModuleId, MonitorId};

    #[test]
    fn only_tooltips_are_click_through() {
        let target = || PopupTarget::new(ModuleId::new(1), MonitorId::new("DP-1"));
        assert!(is_click_through(&FloatingKind::Tooltip));
        assert!(!is_click_through(&FloatingKind::Popup(target())));
        assert!(!is_click_through(&FloatingKind::Panel(target())));
    }
}
