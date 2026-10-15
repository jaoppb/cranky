//! `wl_pointer` event handling: routes each event to the module under the
//! pointer, including children composited into a floating surface's buffer.
//!
//! Every event re-routes the current position, so `pointer_target` always
//! names whoever last got a `PointerEnter`; crossings between the popup's own
//! area and a child are synthesized as `PointerLeave`/`PointerEnter` pairs.

use super::state::WaylandState;
use crate::features::layout_engine::domain::FloatingKind;
use crate::shared::events::core::{
    InteractionEvent, PointerButton, PointerEvent, PointerSerial, ScrollAxis, ScrollDelta,
    SurfaceKind,
};
use crate::shared::primitives::geometry::Position;
use crate::shared::wayland::composite::route::{PointerTarget, crossing, hit};
use wayland_client::WEnum;
use wayland_client::protocol::{wl_pointer, wl_surface::WlSurface};

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
        .and_then(|(kind, floating)| hit(&floating.layers, floating_scale(state, kind), pos));
    if let Some(child) = child {
        let target = (child.module_id, child.monitor_id, SurfaceKind::Bar);
        return Some((target, child.local));
    }
    let target = state.surface_to_id.get(surface)?;
    Some((target.clone(), pos))
}

fn send(state: &WaylandState, target: &PointerTarget, event: PointerEvent) {
    let (id, monitor, _) = target;
    tracing::debug!(module = %id, monitor = %monitor, ?event, "Forwarding pointer event");
    let _ = state
        .hub
        .pointer_tx()
        .send((*id, monitor.clone(), InteractionEvent::Pointer(event)));
}

/// Routes the current pointer position, sending `PointerLeave` to the old
/// target and `PointerEnter` to the new one when they differ. Returns the
/// new target with its local position.
fn retarget(state: &mut WaylandState) -> Option<(PointerTarget, Position)> {
    let (x, y) = state.pointer_pos;
    let pos = Position::new(f64_to_i32(x), f64_to_i32(y));
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

fn retarget_and_move(state: &mut WaylandState) {
    if let Some((target, pos)) = retarget(state) {
        let surface = target.2;
        send(state, &target, PointerEvent::PointerMotion { surface, pos });
    }
}

pub(super) fn handle_pointer_enter(
    state: &mut WaylandState,
    surface: &WlSurface,
    surface_x: f64,
    surface_y: f64,
) {
    tracing::debug!(surface_x, surface_y, "wl_pointer Enter");
    state.pointer_surface = Some(surface.clone());
    state.pointer_pos = (surface_x, surface_y);
    retarget_and_move(state);
}

pub(super) fn handle_pointer_motion(state: &mut WaylandState, surface_x: f64, surface_y: f64) {
    state.pointer_pos = (surface_x, surface_y);
    retarget_and_move(state);
}

pub(super) fn handle_pointer_leave(state: &mut WaylandState) {
    tracing::debug!("wl_pointer Leave");
    state.pointer_surface = None;
    if let Some(target) = state.pointer_target.take() {
        let surface = target.2;
        send(state, &target, PointerEvent::PointerLeave { surface });
    }
}

/// Records the serial (`xdg_popup.grab` needs it even when nothing is hit),
/// then sends press, or release followed by click, to the current target.
pub(super) fn handle_pointer_button(
    state: &mut WaylandState,
    button: u32,
    button_state: WEnum<wl_pointer::ButtonState>,
    serial: u32,
) {
    tracing::debug!(button, ?button_state, serial, "wl_pointer Button");
    state.last_button_serial = Some(PointerSerial::new(serial));
    let pressed = match button_state {
        WEnum::Value(wl_pointer::ButtonState::Pressed) => true,
        WEnum::Value(wl_pointer::ButtonState::Released) => false,
        _ => return,
    };
    let Some((target, pos)) = retarget(state) else {
        return;
    };
    let (surface, button) = (target.2, PointerButton::from_raw(button));
    if pressed {
        send(
            state,
            &target,
            PointerEvent::ButtonPress {
                surface,
                button,
                pos,
            },
        );
    } else {
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
}

pub(super) fn handle_pointer_axis(
    state: &mut WaylandState,
    axis: WEnum<wl_pointer::Axis>,
    value: f64,
) {
    tracing::debug!(?axis, value, "wl_pointer Axis");
    let Some((target, _)) = retarget(state) else {
        return;
    };
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
        InteractionEvent, PointerButton, PointerEvent, PointerSerial, ScrollDelta, SurfaceKind,
    };
    use crate::shared::primitives::{ModuleId, MonitorId, Position, Size};
    use crate::shared::wayland::adapters::wayland::state::WaylandState;
    use crate::shared::wayland::adapters::wayland::tests::fixture::{Fake, popup_kind, solid};
    use tokio::sync::broadcast::Receiver;
    use wayland_client::WEnum;
    use wayland_client::protocol::wl_pointer::{Axis, ButtonState};
    use wayland_client::protocol::wl_surface::WlSurface;

    type Event = (ModuleId, MonitorId, InteractionEvent);

    /// `popup_kind()`'s owner: module 1 on DP-1.
    const OWNER: u32 = 1;
    const CHILD: u32 = 9;
    const BAR_MODULE: u32 = 3;
    const LEFT: u32 = 0x110;
    const PRESSED: WEnum<ButtonState> = WEnum::Value(ButtonState::Pressed);
    const RELEASED: WEnum<ButtonState> = WEnum::Value(ButtonState::Released);

    /// A 20×10 popup owned by module 1 on DP-1, whose bar has `scale`, with
    /// module 9 composited as a 4×4 logical layer at (10, 2).
    fn popup(fake: &Fake, scale: u32) -> (WaylandState, WlSurface) {
        let mut state = fake.state(PopupEmbed::Composite);
        state
            .bars
            .push(fake.bar("DP-1", i32::try_from(scale).unwrap_or(1)));
        let mut floating = fake.floating(Size::new(20, 10), solid(20, 10, [0; 4]));
        let side = 4u32.saturating_mul(scale);
        floating.layers.upsert(
            ModuleId::new(CHILD),
            MonitorId::new("DP-1"),
            Position::new(10, 2),
            solid(side, side, [1; 4]),
        );
        let surface = floating.surface.clone();
        state.surface_to_id.insert(
            surface.clone(),
            (
                ModuleId::new(OWNER),
                MonitorId::new("DP-1"),
                SurfaceKind::Popup,
            ),
        );
        state.floating_surfaces.insert(popup_kind(), floating);
        (state, surface)
    }

    /// Registers a bar module subsurface for module 3 on HDMI-A-1.
    fn bar_module(fake: &Fake, state: &mut WaylandState) -> WlSurface {
        let surface = fake.surface();
        state.surface_to_id.insert(
            surface.clone(),
            (
                ModuleId::new(BAR_MODULE),
                MonitorId::new("HDMI-A-1"),
                SurfaceKind::Bar,
            ),
        );
        surface
    }

    fn drain(rx: &mut Receiver<Event>) -> Vec<Event> {
        std::iter::from_fn(|| rx.try_recv().ok()).collect()
    }

    fn ev(id: u32, monitor: &str, event: PointerEvent) -> Event {
        (
            ModuleId::new(id),
            MonitorId::new(monitor),
            InteractionEvent::Pointer(event),
        )
    }

    fn owner(event: PointerEvent) -> Event {
        ev(OWNER, "DP-1", event)
    }

    fn child(event: PointerEvent) -> Event {
        ev(CHILD, "DP-1", event)
    }

    fn enter(surface: SurfaceKind) -> PointerEvent {
        PointerEvent::PointerEnter { surface }
    }

    fn leave(surface: SurfaceKind) -> PointerEvent {
        PointerEvent::PointerLeave { surface }
    }

    fn motion(surface: SurfaceKind, x: i32, y: i32) -> PointerEvent {
        PointerEvent::PointerMotion {
            surface,
            pos: Position::new(x, y),
        }
    }

    fn clicks(surface: SurfaceKind, x: i32, y: i32) -> [PointerEvent; 3] {
        let (button, pos) = (PointerButton::Left, Position::new(x, y));
        [
            PointerEvent::ButtonPress {
                surface,
                button,
                pos,
            },
            PointerEvent::ButtonRelease {
                surface,
                button,
                pos,
            },
            PointerEvent::Click {
                surface,
                button,
                pos,
            },
        ]
    }

    fn click(state: &mut WaylandState) {
        handle_pointer_button(state, LEFT, PRESSED, 7);
        handle_pointer_button(state, LEFT, RELEASED, 8);
    }

    #[test]
    fn dispatch_pointer_owner_area_to_child_crosses() {
        let fake = Fake::new();
        let (mut state, surface) = popup(&fake, 1);
        let mut rx = state.hub.pointer_rx();
        handle_pointer_enter(&mut state, &surface, 1.0, 1.0);
        assert_eq!(
            drain(&mut rx),
            vec![
                owner(enter(SurfaceKind::Popup)),
                owner(motion(SurfaceKind::Popup, 1, 1))
            ]
        );
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
    fn dispatch_pointer_click_on_child_is_local_to_child() {
        let fake = Fake::new();
        let (mut state, surface) = popup(&fake, 1);
        let mut rx = state.hub.pointer_rx();
        handle_pointer_enter(&mut state, &surface, 13.0, 5.0);
        drain(&mut rx);
        click(&mut state);
        assert_eq!(
            drain(&mut rx),
            clicks(SurfaceKind::Bar, 3, 3).map(child).to_vec()
        );
        assert_eq!(state.last_button_serial, Some(PointerSerial::new(8)));
    }

    #[test]
    fn dispatch_pointer_click_on_owner_area_goes_to_owner() {
        let fake = Fake::new();
        let (mut state, surface) = popup(&fake, 1);
        let mut rx = state.hub.pointer_rx();
        handle_pointer_enter(&mut state, &surface, 5.0, 6.0);
        drain(&mut rx);
        click(&mut state);
        assert_eq!(
            drain(&mut rx),
            clicks(SurfaceKind::Popup, 5, 6).map(owner).to_vec()
        );
    }

    #[test]
    fn dispatch_pointer_scroll_on_child_goes_to_child() {
        let fake = Fake::new();
        let (mut state, surface) = popup(&fake, 1);
        let mut rx = state.hub.pointer_rx();
        handle_pointer_enter(&mut state, &surface, 11.0, 3.0);
        drain(&mut rx);
        handle_pointer_axis(&mut state, WEnum::Value(Axis::VerticalScroll), 2.5);
        assert_eq!(
            drain(&mut rx),
            vec![child(PointerEvent::Scroll {
                surface: SurfaceKind::Bar,
                axis: crate::shared::events::core::ScrollAxis::Vertical,
                amount: ScrollDelta::new(2.5),
            })]
        );
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
        assert_eq!(
            routed,
            Some((
                (
                    ModuleId::new(CHILD),
                    MonitorId::new("DP-1"),
                    SurfaceKind::Bar
                ),
                Position::new(3, 3)
            ))
        );
    }

    #[test]
    fn dispatch_pointer_bar_module_routing_is_unchanged() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        let surface = bar_module(&fake, &mut state);
        let mut rx = state.hub.pointer_rx();
        handle_pointer_enter(&mut state, &surface, 5.0, 6.0);
        click(&mut state);
        handle_pointer_leave(&mut state);
        let bar = |event| ev(BAR_MODULE, "HDMI-A-1", event);
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
        let bar = |event| ev(BAR_MODULE, "HDMI-A-1", event);
        assert_eq!(
            drain(&mut rx),
            vec![
                child(leave(SurfaceKind::Bar)),
                bar(enter(SurfaceKind::Bar)),
                bar(motion(SurfaceKind::Bar, 4, 4)),
            ]
        );
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
