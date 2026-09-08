use super::state::WaylandState;
use crate::shared::events::core::{
    InteractionEvent, PointerButton, PointerEvent, ScrollAxis, ScrollDelta,
};
use crate::shared::primitives::geometry::Position;
use wayland_client::protocol::{
    wl_pointer::{self, WlPointer},
    wl_seat::{self, WlSeat},
};
use wayland_client::{Connection, Dispatch, QueueHandle};

impl Dispatch<WlSeat, ()> for WaylandState {
    fn event(
        state: &mut Self,
        proxy: &WlSeat,
        event: wl_seat::Event,
        (): &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities { capabilities } = event {
            let caps =
                wayland_client::protocol::wl_seat::Capability::from_bits(capabilities.into())
                    .unwrap_or(wayland_client::protocol::wl_seat::Capability::empty());
            if caps.contains(wayland_client::protocol::wl_seat::Capability::Pointer)
                && state.pointer.is_none()
            {
                state.pointer = Some(proxy.get_pointer(qh, ()));
            }
        }
    }
}

fn f64_to_i32(val: f64) -> i32 {
    i32::try_from(crate::utils::f64_to_i64(val)).unwrap_or(0)
}

fn handle_pointer_enter(
    state: &mut WaylandState,
    surface: &wayland_client::protocol::wl_surface::WlSurface,
    surface_x: f64,
    surface_y: f64,
) {
    tracing::debug!(surface_x, surface_y, "wl_pointer Enter");
    state.pointer_surface = Some(surface.clone());
    state.pointer_pos = (surface_x, surface_y);
    if let Some((id, mon_id, kind)) = state.surface_to_id.get(surface) {
        tracing::debug!(module = %id, monitor = %mon_id, "Forwarding PointerEnter");
        let _ = state.hub.pointer_tx().send((
            *id,
            mon_id.clone(),
            InteractionEvent::Pointer(PointerEvent::PointerEnter { surface: *kind }),
        ));
    }
}

fn handle_pointer_leave(state: &mut WaylandState) {
    tracing::debug!("wl_pointer Leave");
    if let Some(surface) = state.pointer_surface.take()
        && let Some((id, mon_id, kind)) = state.surface_to_id.get(&surface)
    {
        tracing::debug!(module = %id, monitor = %mon_id, "Forwarding PointerLeave");
        let _ = state.hub.pointer_tx().send((
            *id,
            mon_id.clone(),
            InteractionEvent::Pointer(PointerEvent::PointerLeave { surface: *kind }),
        ));
    }
}

fn handle_pointer_motion(state: &mut WaylandState, surface_x: f64, surface_y: f64) {
    state.pointer_pos = (surface_x, surface_y);
    if let Some(surface) = &state.pointer_surface
        && let Some((id, mon_id, kind)) = state.surface_to_id.get(surface)
    {
        let pos = Position::new(f64_to_i32(surface_x), f64_to_i32(surface_y));
        let _ = state.hub.pointer_tx().send((
            *id,
            mon_id.clone(),
            InteractionEvent::Pointer(PointerEvent::PointerMotion {
                surface: *kind,
                pos,
            }),
        ));
    }
}

fn handle_pointer_button(
    state: &mut WaylandState,
    button: u32,
    button_state: wayland_client::WEnum<wl_pointer::ButtonState>,
    serial: u32,
) {
    tracing::debug!(button, ?button_state, serial, "wl_pointer Button");
    state.last_button_serial = Some(crate::shared::events::core::PointerSerial::new(serial));
    let ptr_button = PointerButton::from_raw(button);
    let pos = Position::new(f64_to_i32(state.pointer_pos.0), f64_to_i32(state.pointer_pos.1));

    let Some(surface) = &state.pointer_surface else { return };
    let Some((id, mon_id, kind)) = state.surface_to_id.get(surface) else { return };

    let event = match button_state {
        wayland_client::WEnum::Value(wl_pointer::ButtonState::Pressed) => {
            tracing::debug!(module = %id, monitor = %mon_id, "Forwarding ButtonPress");
            PointerEvent::ButtonPress {
                surface: *kind,
                button: ptr_button,
                pos,
            }
        }
        wayland_client::WEnum::Value(wl_pointer::ButtonState::Released) => {
            tracing::debug!(module = %id, monitor = %mon_id, "Forwarding ButtonRelease & Click");
            let _ = state.hub.pointer_tx().send((
                *id,
                mon_id.clone(),
                InteractionEvent::Pointer(PointerEvent::ButtonRelease {
                    surface: *kind,
                    button: ptr_button,
                    pos,
                }),
            ));
            PointerEvent::Click {
                surface: *kind,
                button: ptr_button,
                pos,
            }
        }
        _ => return,
    };
    let _ = state.hub.pointer_tx().send((*id, mon_id.clone(), InteractionEvent::Pointer(event)));
}

fn handle_pointer_axis(
    state: &WaylandState,
    axis: wayland_client::WEnum<wl_pointer::Axis>,
    value: f64,
) {
    tracing::debug!(?axis, value, "wl_pointer Axis");
    if let Some(surface) = &state.pointer_surface
        && let Some((id, mon_id, kind)) = state.surface_to_id.get(surface)
    {
        let scroll_axis = match axis {
            wayland_client::WEnum::Value(wl_pointer::Axis::VerticalScroll) => ScrollAxis::Vertical,
            _ => ScrollAxis::Horizontal,
        };
        tracing::debug!(module = %id, monitor = %mon_id, "Forwarding Scroll");
        let _ = state.hub.pointer_tx().send((
            *id,
            mon_id.clone(),
            InteractionEvent::Pointer(PointerEvent::Scroll {
                surface: *kind,
                axis: scroll_axis,
                amount: ScrollDelta::new(value),
            }),
        ));
    }
}

impl Dispatch<WlPointer, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _proxy: &WlPointer,
        event: wl_pointer::Event,
        (): &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Enter {
                surface,
                surface_x,
                surface_y,
                ..
            } => handle_pointer_enter(state, &surface, surface_x, surface_y),
            wl_pointer::Event::Leave { .. } => handle_pointer_leave(state),
            wl_pointer::Event::Motion { surface_x, surface_y, .. } => {
                handle_pointer_motion(state, surface_x, surface_y);
            }
            wl_pointer::Event::Button { button, state: btn_state, serial, .. } => {
                handle_pointer_button(state, button, btn_state, serial);
            }
            wl_pointer::Event::Axis { axis, value, .. } => handle_pointer_axis(state, axis, value),
            _ => {}
        }
    }
}
