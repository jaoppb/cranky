//! `wl_pointer` event handling: routes each event to the module under the
//! pointer, including children composited into a floating surface's buffer.
//!
//! Every event re-routes the current position, so `pointer_target` always
//! names whoever last got a `PointerEnter`; crossings between the popup's own
//! area and a child are synthesized as `PointerLeave`/`PointerEnter` pairs.
//! While a button is held, routing is frozen instead (see
//! `dispatch_pointer_button`).

use super::dispatch_pointer_button::grabbed;
use super::state::WaylandState;
use crate::features::layout_engine::domain::FloatingKind;
use crate::shared::events::core::{InteractionEvent, PointerEvent, SurfaceKind};
use crate::shared::primitives::geometry::Position;
use crate::shared::wayland::composite::route::{PointerTarget, crossing, hit};
use wayland_client::protocol::wl_surface::WlSurface;

fn f64_to_i32(val: f64) -> i32 {
    i32::try_from(crate::utils::f64_to_i64(val)).unwrap_or(0)
}

/// The scale of the bar on `kind`'s target monitor, 1 when there is none.
fn floating_scale(state: &WaylandState, kind: &FloatingKind) -> i32 {
    let monitor = match kind {
        FloatingKind::Popup(target) | FloatingKind::Panel(target) => target.monitor_id(),
        FloatingKind::Tooltip => return 1,
    };
    state
        .bars
        .iter()
        .find(|bar| bar.output_name == monitor.as_str())
        .map_or(1, |bar| bar.scale)
}

/// The module that logical `pos` on `surface` addresses, with the position
/// it sees: a composited child gets child-local coordinates, anything else
/// is the surface's own `surface_to_id` entry with `pos` unchanged.
pub(super) fn route(
    state: &WaylandState,
    surface: &WlSurface,
    pos: Position,
) -> Option<(PointerTarget, Position)> {
    let child = state
        .floating_surfaces
        .iter()
        .find(|(_, floating)| &floating.surface == surface)
        // Tooltips get no pointer routing, composited children included.
        .filter(|(kind, _)| !matches!(kind, FloatingKind::Tooltip))
        .and_then(|(kind, floating)| hit(&floating.layers, floating_scale(state, kind), pos));
    if let Some(child) = child {
        let target = (child.module_id, child.monitor_id, SurfaceKind::Bar);
        return Some((target, child.local));
    }
    let target = state.surface_to_id.get(surface)?;
    Some((target.clone(), pos))
}

pub(super) fn send(state: &WaylandState, target: &PointerTarget, event: PointerEvent) {
    let (id, monitor, _) = target;
    tracing::debug!(module = %id, monitor = %monitor, ?event, "Forwarding pointer event");
    let _ = state
        .hub
        .pointer_tx()
        .send((*id, monitor.clone(), InteractionEvent::Pointer(event)));
}

/// The pointer's surface-local logical position.
pub(super) fn pointer_pos(state: &WaylandState) -> Position {
    let (x, y) = state.pointer_pos;
    Position::new(f64_to_i32(x), f64_to_i32(y))
}

/// Routes the current pointer position, sending `PointerLeave` to the old
/// target and `PointerEnter` to the new one when they differ. Returns the
/// new target with its local position.
pub(super) fn retarget(state: &mut WaylandState) -> Option<(PointerTarget, Position)> {
    let pos = pointer_pos(state);
    let next = state
        .pointer_surface
        .as_ref()
        .and_then(|surface| route(state, surface, pos));
    let next_target = next.as_ref().map(|(target, _)| target);
    let cross = crossing(state.pointer_target.as_ref(), next_target);
    if let Some(old) = &cross.leave {
        send(state, old, PointerEvent::PointerLeave { surface: old.2 });
    }
    if let Some(new) = &cross.enter {
        send(state, new, PointerEvent::PointerEnter { surface: new.2 });
    }
    state.pointer_target = next_target.cloned();
    next
}

/// Whether `target` is a composited child rather than the pointer surface's
/// own module.
fn is_composite_child(state: &WaylandState, target: &PointerTarget) -> bool {
    let own = state
        .pointer_surface
        .as_ref()
        .and_then(|s| state.surface_to_id.get(s));
    own != Some(target)
}

/// Retargets after an Enter not driven by motion (`wl_pointer.enter`, the end
/// of a grab). A newly entered composite child also gets a Motion, its only
/// source for the entry position; any other target gets just the
/// `PointerEnter`, exactly as before compositing.
pub(super) fn retarget_entering(state: &mut WaylandState) {
    let before = state.pointer_target.clone();
    if let Some((target, pos)) = retarget(state)
        && before.as_ref() != Some(&target)
        && is_composite_child(state, &target)
    {
        let surface = target.2;
        send(state, &target, PointerEvent::PointerMotion { surface, pos });
    }
}

/// A new Enter means the compositor ended any implicit grab.
pub(super) fn handle_pointer_enter(
    state: &mut WaylandState,
    surface: &WlSurface,
    surface_x: f64,
    surface_y: f64,
) {
    tracing::debug!(surface_x, surface_y, "wl_pointer Enter");
    state.pointer_grab.cancel();
    state.pointer_surface = Some(surface.clone());
    state.pointer_pos = (surface_x, surface_y);
    retarget_entering(state);
}

/// While a button is held, motion goes to the grabbed target relative to its
/// press-time origin; otherwise it retargets first.
pub(super) fn handle_pointer_motion(state: &mut WaylandState, surface_x: f64, surface_y: f64) {
    state.pointer_pos = (surface_x, surface_y);
    let routed = if state.pointer_grab.is_active() {
        grabbed(state)
    } else {
        retarget(state)
    };
    if let Some((target, pos)) = routed {
        let surface = target.2;
        send(state, &target, PointerEvent::PointerMotion { surface, pos });
    }
}

/// A Leave during a hold also ends the grab: the grabbed target gets its
/// `PointerLeave` once, and the next Enter routes from scratch.
pub(super) fn handle_pointer_leave(state: &mut WaylandState) {
    tracing::debug!("wl_pointer Leave");
    state.pointer_grab.cancel();
    state.pointer_surface = None;
    if let Some(target) = state.pointer_target.take() {
        let surface = target.2;
        send(state, &target, PointerEvent::PointerLeave { surface });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::config::domain::PopupEmbed;
    use crate::shared::primitives::{ModuleId, MonitorId};
    use crate::shared::wayland::adapters::wayland::dispatch_pointer_button::handle_pointer_button;
    use crate::shared::wayland::adapters::wayland::tests::fixture::pointer::{
        CHILD, LEFT, OWNER, PRESSED, RELEASED, bar, bar_module, child, clicks, drain, enter, leave,
        motion, owner, popup,
    };
    use crate::shared::wayland::adapters::wayland::tests::fixture::{Fake, popup_kind};

    #[test]
    fn dispatch_pointer_enter_on_owner_area_sends_only_enter() {
        let fake = Fake::new();
        let (mut state, surface) = popup(&fake, 1);
        let mut rx = state.hub.pointer_rx();
        handle_pointer_enter(&mut state, &surface, 1.0, 1.0);
        assert_eq!(drain(&mut rx), vec![owner(enter(SurfaceKind::Popup))]);
    }

    #[test]
    fn dispatch_pointer_enter_on_child_sends_enter_then_local_motion() {
        let fake = Fake::new();
        let (mut state, surface) = popup(&fake, 1);
        let mut rx = state.hub.pointer_rx();
        handle_pointer_enter(&mut state, &surface, 11.0, 3.0);
        assert_eq!(
            drain(&mut rx),
            vec![
                child(enter(SurfaceKind::Bar)),
                child(motion(SurfaceKind::Bar, 1, 1))
            ]
        );
    }

    #[test]
    fn dispatch_pointer_owner_area_to_child_crosses() {
        let fake = Fake::new();
        let (mut state, surface) = popup(&fake, 1);
        let mut rx = state.hub.pointer_rx();
        handle_pointer_enter(&mut state, &surface, 1.0, 1.0);
        drain(&mut rx);
        handle_pointer_motion(&mut state, 11.0, 3.0);
        assert_eq!(
            drain(&mut rx),
            vec![
                owner(leave(SurfaceKind::Popup)),
                child(enter(SurfaceKind::Bar)),
                child(motion(SurfaceKind::Bar, 1, 1)),
            ]
        );
        handle_pointer_motion(&mut state, 12.0, 4.0);
        assert_eq!(drain(&mut rx), vec![child(motion(SurfaceKind::Bar, 2, 2))]);
        handle_pointer_motion(&mut state, 2.0, 2.0);
        assert_eq!(
            drain(&mut rx),
            vec![
                child(leave(SurfaceKind::Bar)),
                owner(enter(SurfaceKind::Popup)),
                owner(motion(SurfaceKind::Popup, 2, 2)),
            ]
        );
    }

    #[test]
    fn dispatch_pointer_leaving_popup_from_child_leaves_child_once() {
        let fake = Fake::new();
        let (mut state, surface) = popup(&fake, 1);
        let mut rx = state.hub.pointer_rx();
        handle_pointer_enter(&mut state, &surface, 11.0, 3.0);
        drain(&mut rx);
        handle_pointer_leave(&mut state);
        assert_eq!(drain(&mut rx), vec![child(leave(SurfaceKind::Bar))]);
        assert_eq!(state.pointer_target, None);
        handle_pointer_leave(&mut state);
        handle_pointer_motion(&mut state, 11.0, 3.0);
        assert_eq!(drain(&mut rx), vec![]);
    }

    #[test]
    fn dispatch_pointer_route_scales_layers_by_target_monitor_bar() {
        let fake = Fake::new();
        let (state, surface) = popup(&fake, 2);
        let route_id =
            |x, y| route(&state, &surface, Position::new(x, y)).map(|(t, p)| (t.0.value(), p));
        assert_eq!(route_id(13, 5), Some((CHILD, Position::new(3, 3))));
        assert_eq!(route_id(15, 7), Some((OWNER, Position::new(15, 7))));
    }

    #[test]
    fn dispatch_pointer_route_scale_falls_back_to_one_without_bar() {
        let fake = Fake::new();
        let (mut state, surface) = popup(&fake, 1);
        state.bars.clear();
        let routed = route(&state, &surface, Position::new(13, 5));
        let target = (
            ModuleId::new(CHILD),
            MonitorId::new("DP-1"),
            SurfaceKind::Bar,
        );
        assert_eq!(routed, Some((target, Position::new(3, 3))));
    }

    /// The pre-composite sequence: Enter carries no Motion, and nothing
    /// else changes for a module on its own surface.
    #[test]
    fn dispatch_pointer_bar_module_keeps_legacy_event_sequence() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        let surface = bar_module(&fake, &mut state);
        let mut rx = state.hub.pointer_rx();
        handle_pointer_enter(&mut state, &surface, 5.0, 6.0);
        handle_pointer_motion(&mut state, 5.0, 6.0);
        handle_pointer_button(&mut state, LEFT, PRESSED, 1);
        handle_pointer_button(&mut state, LEFT, RELEASED, 2);
        handle_pointer_leave(&mut state);
        let mut expected = vec![
            bar(enter(SurfaceKind::Bar)),
            bar(motion(SurfaceKind::Bar, 5, 6)),
        ];
        expected.extend(clicks(SurfaceKind::Bar, 5, 6).map(bar));
        expected.push(bar(leave(SurfaceKind::Bar)));
        assert_eq!(drain(&mut rx), expected);
    }

    #[test]
    fn dispatch_pointer_enter_new_surface_without_leave_crosses() {
        let fake = Fake::new();
        let (mut state, popup_surface) = popup(&fake, 1);
        let bar_surface = bar_module(&fake, &mut state);
        let mut rx = state.hub.pointer_rx();
        handle_pointer_enter(&mut state, &popup_surface, 11.0, 3.0);
        drain(&mut rx);
        handle_pointer_enter(&mut state, &bar_surface, 4.0, 4.0);
        assert_eq!(
            drain(&mut rx),
            vec![child(leave(SurfaceKind::Bar)), bar(enter(SurfaceKind::Bar))]
        );
    }

    /// Tooltips never had pointer routing; a layer composited into one must
    /// not grow it.
    #[test]
    fn dispatch_pointer_tooltip_layers_get_no_routing() {
        let fake = Fake::new();
        let (mut state, surface) = popup(&fake, 2);
        state.surface_to_id.remove(&surface);
        let floating = state.floating_surfaces.remove(&popup_kind());
        let floating = floating.expect("popup floating surface");
        state
            .floating_surfaces
            .insert(FloatingKind::Tooltip, floating);

        assert_eq!(route(&state, &surface, Position::new(13, 5)), None);
    }
}
