//! Pointer buttons and scrolling, under an emulated implicit grab.
//!
//! A composited child has no surface of its own for the compositor's
//! implicit grab to hold, so while any button is held the target is frozen
//! here instead.

use super::dispatch_pointer::{pointer_pos, retarget, retarget_entering, send};
use super::state::WaylandState;
use crate::shared::events::core::{
    PointerButton, PointerEvent, PointerSerial, ScrollAxis, ScrollDelta,
};
use crate::shared::primitives::geometry::Position;
use crate::shared::wayland::composite::route::PointerTarget;
use std::collections::BTreeSet;
use wayland_client::WEnum;
use wayland_client::protocol::wl_pointer;

/// The held buttons and, while any is, the frozen target's origin on the
/// pointer surface: the child layer's origin at press time, or (0, 0) for a
/// module on its own surface.
#[derive(Debug)]
pub(crate) struct PointerGrab {
    held: BTreeSet<u32>,
    origin: Position,
}

impl Default for PointerGrab {
    fn default() -> Self {
        Self {
            held: BTreeSet::new(),
            origin: Position::new(0, 0),
        }
    }
}

impl PointerGrab {
    pub(crate) fn is_active(&self) -> bool {
        !self.held.is_empty()
    }

    /// Starts a grab frozen at `origin`; a grab already active keeps its own.
    fn begin(&mut self, origin: Position) {
        if self.held.is_empty() {
            self.origin = origin;
        }
    }

    fn hold(&mut self, button: u32) {
        self.held.insert(button);
    }

    /// Returns whether this release ended the grab; releasing a button that
    /// is not held changes nothing.
    fn release(&mut self, button: u32) -> bool {
        self.held.remove(&button) && self.held.is_empty()
    }

    pub(crate) fn cancel(&mut self) {
        self.held.clear();
    }

    /// `pos` relative to the frozen target's origin; may be negative or past
    /// the target's size, which the target hit-tests itself.
    pub(crate) const fn local(&self, pos: Position) -> Position {
        Position::new(
            pos.x().saturating_sub(self.origin.x()),
            pos.y().saturating_sub(self.origin.y()),
        )
    }
}

/// The frozen target with the pointer relative to its press-time origin.
pub(super) fn grabbed(state: &WaylandState) -> Option<(PointerTarget, Position)> {
    let pos = state.pointer_grab.local(pointer_pos(state));
    state.pointer_target.clone().map(|target| (target, pos))
}

/// The first press retargets and freezes; later presses join the grab.
fn press(state: &mut WaylandState, button: u32) {
    if !state.pointer_grab.is_active() {
        let pos = pointer_pos(state);
        let local = retarget(state).map_or(pos, |(_, local)| local);
        let origin = Position::new(
            pos.x().saturating_sub(local.x()),
            pos.y().saturating_sub(local.y()),
        );
        state.pointer_grab.begin(origin);
    }
    state.pointer_grab.hold(button);
    if let Some((target, pos)) = grabbed(state) {
        let (surface, button) = (target.2, PointerButton::from_raw(button));
        send(
            state,
            &target,
            PointerEvent::ButtonPress {
                surface,
                button,
                pos,
            },
        );
    }
}

/// Release + click go to the frozen target; the last held button's release
/// then unfreezes and retargets at the current position. A release with no
/// grab at all (pressed before the pointer arrived) routes normally.
fn release(state: &mut WaylandState, button: u32) {
    let frozen = state.pointer_grab.is_active();
    let ended = state.pointer_grab.release(button);
    let routed = if frozen {
        grabbed(state)
    } else {
        retarget(state)
    };
    if let Some((target, pos)) = routed {
        let (surface, button) = (target.2, PointerButton::from_raw(button));
        send(
            state,
            &target,
            PointerEvent::ButtonRelease {
                surface,
                button,
                pos,
            },
        );
        send(
            state,
            &target,
            PointerEvent::Click {
                surface,
                button,
                pos,
            },
        );
    }
    if ended {
        retarget_entering(state);
    }
}

/// Records the serial (`xdg_popup.grab` needs it even when nothing is hit),
/// then handles the press or release.
pub(super) fn handle_pointer_button(
    state: &mut WaylandState,
    button: u32,
    button_state: WEnum<wl_pointer::ButtonState>,
    serial: u32,
) {
    tracing::debug!(button, ?button_state, serial, "wl_pointer Button");
    state.last_button_serial = Some(PointerSerial::new(serial));
    match button_state {
        WEnum::Value(wl_pointer::ButtonState::Pressed) => press(state, button),
        WEnum::Value(wl_pointer::ButtonState::Released) => release(state, button),
        _ => {}
    }
}

pub(super) fn handle_pointer_axis(
    state: &mut WaylandState,
    axis: WEnum<wl_pointer::Axis>,
    value: f64,
) {
    tracing::debug!(?axis, value, "wl_pointer Axis");
    let routed = if state.pointer_grab.is_active() {
        grabbed(state)
    } else {
        retarget(state)
    };
    let Some((target, _)) = routed else { return };
    let axis = match axis {
        WEnum::Value(wl_pointer::Axis::VerticalScroll) => ScrollAxis::Vertical,
        _ => ScrollAxis::Horizontal,
    };
    let (surface, amount) = (target.2, ScrollDelta::new(value));
    send(
        state,
        &target,
        PointerEvent::Scroll {
            surface,
            axis,
            amount,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::config::domain::PopupEmbed;
    use crate::shared::events::core::{
        PointerEvent, PointerSerial, ScrollAxis, ScrollDelta, SurfaceKind,
    };
    use crate::shared::primitives::{ModuleId, MonitorId, Position};
    use crate::shared::wayland::adapters::wayland::dispatch_pointer::{
        handle_pointer_enter, handle_pointer_leave, handle_pointer_motion,
    };
    use crate::shared::wayland::adapters::wayland::state::WaylandState;
    use crate::shared::wayland::adapters::wayland::tests::fixture::pointer::{
        CHILD, LEFT, PRESSED, RELEASED, RIGHT, bar, bar_module, child, clicks, drain, enter, leave,
        motion, owner, popup, press, release,
    };
    use crate::shared::wayland::adapters::wayland::tests::fixture::{Fake, popup_kind, solid};
    use wayland_client::WEnum;
    use wayland_client::protocol::wl_pointer::Axis;

    fn scroll(surface: SurfaceKind) -> PointerEvent {
        PointerEvent::Scroll {
            surface,
            axis: ScrollAxis::Vertical,
            amount: ScrollDelta::new(2.5),
        }
    }

    fn scroll_down(state: &mut WaylandState) {
        handle_pointer_axis(state, WEnum::Value(Axis::VerticalScroll), 2.5);
    }

    #[test]
    fn dispatch_pointer_button_click_on_child_is_local_to_child() {
        let fake = Fake::new();
        let (mut state, surface) = popup(&fake, 1);
        let mut rx = state.hub.pointer_rx();
        handle_pointer_enter(&mut state, &surface, 13.0, 5.0);
        drain(&mut rx);
        handle_pointer_button(&mut state, LEFT, PRESSED, 7);
        handle_pointer_button(&mut state, LEFT, RELEASED, 8);
        assert_eq!(
            drain(&mut rx),
            clicks(SurfaceKind::Bar, 3, 3).map(child).to_vec()
        );
        assert_eq!(state.last_button_serial, Some(PointerSerial::new(8)));
    }

    #[test]
    fn dispatch_pointer_button_click_on_owner_area_goes_to_owner() {
        let fake = Fake::new();
        let (mut state, surface) = popup(&fake, 1);
        let mut rx = state.hub.pointer_rx();
        handle_pointer_enter(&mut state, &surface, 5.0, 6.0);
        drain(&mut rx);
        handle_pointer_button(&mut state, LEFT, PRESSED, 7);
        handle_pointer_button(&mut state, LEFT, RELEASED, 8);
        assert_eq!(
            drain(&mut rx),
            clicks(SurfaceKind::Popup, 5, 6).map(owner).to_vec()
        );
    }

    /// The implicit grab: a drag off the pressed child keeps every event on
    /// the child, relative to its origin, until the release ends it.
    #[test]
    fn dispatch_pointer_button_drag_off_child_stays_on_child() {
        let fake = Fake::new();
        let (mut state, surface) = popup(&fake, 1);
        let mut rx = state.hub.pointer_rx();
        handle_pointer_enter(&mut state, &surface, 11.0, 3.0);
        drain(&mut rx);
        handle_pointer_button(&mut state, LEFT, PRESSED, 7);
        handle_pointer_motion(&mut state, 2.0, 2.0);
        scroll_down(&mut state);
        assert_eq!(
            drain(&mut rx),
            vec![
                child(press(SurfaceKind::Bar, 1, 1)),
                child(motion(SurfaceKind::Bar, -8, 0)),
                child(scroll(SurfaceKind::Bar)),
            ]
        );
        handle_pointer_button(&mut state, LEFT, RELEASED, 8);
        let mut expected = release(SurfaceKind::Bar, -8, 0).map(child).to_vec();
        expected.extend([
            child(leave(SurfaceKind::Bar)),
            owner(enter(SurfaceKind::Popup)),
        ]);
        assert_eq!(drain(&mut rx), expected);
        handle_pointer_motion(&mut state, 3.0, 2.0);
        assert_eq!(
            drain(&mut rx),
            vec![owner(motion(SurfaceKind::Popup, 3, 2))]
        );
    }

    #[test]
    fn dispatch_pointer_button_release_follows_child_moved_during_hold() {
        let fake = Fake::new();
        let (mut state, surface) = popup(&fake, 1);
        let mut rx = state.hub.pointer_rx();
        handle_pointer_enter(&mut state, &surface, 11.0, 3.0);
        handle_pointer_button(&mut state, LEFT, PRESSED, 7);
        drain(&mut rx);
        if let Some(floating) = state.floating_surfaces.get_mut(&popup_kind()) {
            let (id, monitor) = (ModuleId::new(CHILD), MonitorId::new("DP-1"));
            let smaller = solid(1, 1, [2; 4]);
            floating
                .layers
                .upsert(id, monitor, Position::new(15, 5), smaller);
        }
        handle_pointer_button(&mut state, LEFT, RELEASED, 8);
        let mut expected = release(SurfaceKind::Bar, 1, 1).map(child).to_vec();
        expected.extend([
            child(leave(SurfaceKind::Bar)),
            owner(enter(SurfaceKind::Popup)),
        ]);
        assert_eq!(drain(&mut rx), expected);
    }

    #[test]
    fn dispatch_pointer_button_unmatched_release_neither_panics_nor_freezes() {
        let fake = Fake::new();
        let (mut state, surface) = popup(&fake, 1);
        let mut rx = state.hub.pointer_rx();
        handle_pointer_enter(&mut state, &surface, 11.0, 3.0);
        drain(&mut rx);
        handle_pointer_button(&mut state, LEFT, RELEASED, 7);
        handle_pointer_motion(&mut state, 2.0, 2.0);
        let mut expected = release(SurfaceKind::Bar, 1, 1).map(child).to_vec();
        expected.extend([
            child(leave(SurfaceKind::Bar)),
            owner(enter(SurfaceKind::Popup)),
            owner(motion(SurfaceKind::Popup, 2, 2)),
        ]);
        assert_eq!(drain(&mut rx), expected);
        assert_eq!(state.last_button_serial, Some(PointerSerial::new(7)));
    }

    /// A release of a button that is not held keeps the grab of the one
    /// that is; only the last held button's release ends it.
    #[test]
    fn dispatch_pointer_button_other_release_keeps_grab() {
        let fake = Fake::new();
        let (mut state, surface) = popup(&fake, 1);
        let mut rx = state.hub.pointer_rx();
        handle_pointer_enter(&mut state, &surface, 11.0, 3.0);
        handle_pointer_button(&mut state, LEFT, PRESSED, 7);
        handle_pointer_button(&mut state, RIGHT, RELEASED, 8);
        drain(&mut rx);
        handle_pointer_motion(&mut state, 2.0, 2.0);
        assert_eq!(drain(&mut rx), vec![child(motion(SurfaceKind::Bar, -8, 0))]);
        handle_pointer_button(&mut state, LEFT, RELEASED, 9);
        let mut expected = release(SurfaceKind::Bar, -8, 0).map(child).to_vec();
        expected.extend([
            child(leave(SurfaceKind::Bar)),
            owner(enter(SurfaceKind::Popup)),
        ]);
        assert_eq!(drain(&mut rx), expected);
    }

    /// A compositor Leave during a hold ends the grab: the grabbed target
    /// gets its Leave once and the next Enter routes from scratch.
    #[test]
    fn dispatch_pointer_button_compositor_leave_ends_grab() {
        let fake = Fake::new();
        let (mut state, surface) = popup(&fake, 1);
        let mut rx = state.hub.pointer_rx();
        handle_pointer_enter(&mut state, &surface, 11.0, 3.0);
        handle_pointer_button(&mut state, LEFT, PRESSED, 7);
        drain(&mut rx);
        handle_pointer_leave(&mut state);
        handle_pointer_enter(&mut state, &surface, 2.0, 2.0);
        handle_pointer_motion(&mut state, 3.0, 3.0);
        assert_eq!(
            drain(&mut rx),
            vec![
                child(leave(SurfaceKind::Bar)),
                owner(enter(SurfaceKind::Popup)),
                owner(motion(SurfaceKind::Popup, 3, 3)),
            ]
        );
    }

    #[test]
    fn dispatch_pointer_button_axis_on_bar_module_is_unchanged() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        let surface = bar_module(&fake, &mut state);
        let mut rx = state.hub.pointer_rx();
        handle_pointer_enter(&mut state, &surface, 5.0, 6.0);
        scroll_down(&mut state);
        let expected = vec![bar(enter(SurfaceKind::Bar)), bar(scroll(SurfaceKind::Bar))];
        assert_eq!(drain(&mut rx), expected);
    }

    #[test]
    fn dispatch_pointer_button_serial_recorded_without_target() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        let mut rx = state.hub.pointer_rx();
        handle_pointer_button(&mut state, LEFT, PRESSED, 42);
        assert_eq!(state.last_button_serial, Some(PointerSerial::new(42)));
        let unknown = fake.surface();
        handle_pointer_enter(&mut state, &unknown, 1.0, 1.0);
        handle_pointer_button(&mut state, LEFT, RELEASED, 43);
        assert_eq!(state.last_button_serial, Some(PointerSerial::new(43)));
        assert_eq!(drain(&mut rx), vec![]);
    }
}
