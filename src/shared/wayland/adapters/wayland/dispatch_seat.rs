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
        _data: &(),
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

impl Dispatch<WlPointer, ()> for WaylandState {
    #[allow(clippy::too_many_lines)]
    fn event(
        state: &mut Self,
        _proxy: &WlPointer,
        event: wl_pointer::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Enter {
                surface,
                surface_x,
                surface_y,
                ..
            } => {
                tracing::debug!(surface_x, surface_y, "wl_pointer Enter");
                state.pointer_surface = Some(surface.clone());
                state.pointer_pos = (surface_x, surface_y);
                if let Some((id, mon_id, kind)) = state.surface_to_id.get(&surface) {
                    tracing::debug!(module = %id, monitor = %mon_id, "Forwarding PointerEnter");
                    let _ = state.hub.pointer_tx().send((
                        *id,
                        mon_id.clone(),
                        InteractionEvent::Pointer(PointerEvent::PointerEnter { surface: *kind }),
                    ));
                }
            }
            wl_pointer::Event::Leave { .. } => {
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
            wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                time: _,
            } => {
                state.pointer_pos = (surface_x, surface_y);
                if let Some(surface) = &state.pointer_surface
                    && let Some((id, mon_id, kind)) = state.surface_to_id.get(surface)
                {
                    #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
                    let pos = Position::new(surface_x as i32, surface_y as i32);
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
            wl_pointer::Event::Button {
                button,
                state: button_state,
                serial,
                time: _,
            } => {
                tracing::debug!(button, ?button_state, serial, "wl_pointer Button");
                state.last_button_serial =
                    Some(crate::shared::events::core::PointerSerial::new(serial));
                let ptr_button = PointerButton::from_raw(button);
                #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
                let pos = Position::new(state.pointer_pos.0 as i32, state.pointer_pos.1 as i32);

                if let Some(surface) = &state.pointer_surface
                    && let Some((id, mon_id, kind)) = state.surface_to_id.get(surface)
                {
                    match button_state {
                        wayland_client::WEnum::Value(
                            wayland_client::protocol::wl_pointer::ButtonState::Pressed,
                        ) => {
                            tracing::debug!(module = %id, monitor = %mon_id, "Forwarding ButtonPress");
                            let _ = state.hub.pointer_tx().send((
                                *id,
                                mon_id.clone(),
                                InteractionEvent::Pointer(PointerEvent::ButtonPress {
                                    surface: *kind,
                                    button: ptr_button,
                                    pos,
                                }),
                            ));
                        }
                        wayland_client::WEnum::Value(
                            wayland_client::protocol::wl_pointer::ButtonState::Released,
                        ) => {
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
                            let _ = state.hub.pointer_tx().send((
                                *id,
                                mon_id.clone(),
                                InteractionEvent::Pointer(PointerEvent::Click {
                                    surface: *kind,
                                    button: ptr_button,
                                    pos,
                                }),
                            ));
                        }
                        _ => {}
                    }
                }
            }
            wl_pointer::Event::Axis {
                axis,
                value,
                time: _,
            } => {
                tracing::debug!(?axis, value, "wl_pointer Axis");
                if let Some(surface) = &state.pointer_surface
                    && let Some((id, mon_id, kind)) = state.surface_to_id.get(surface)
                {
                    let scroll_axis = match axis {
                        wayland_client::WEnum::Value(
                            wayland_client::protocol::wl_pointer::Axis::VerticalScroll,
                        ) => ScrollAxis::Vertical,
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
            _ => {}
        }
    }
}
