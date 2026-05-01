use crate::core::CrankyState;
use crate::modules::Event;
use wayland_client::protocol::wl_pointer::{self, WlPointer};
use wayland_client::{Connection, Dispatch, QueueHandle};

impl Dispatch<WlPointer, ()> for CrankyState {
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
                state.pointer_surface = Some(surface);
                state.pointer_pos = (surface_x, surface_y);
                state.dispatch_to_surface(Event::PointerEnter);
            }
            wl_pointer::Event::Leave { .. } => {
                state.dispatch_to_surface(Event::PointerLeave);
                state.pointer_surface = None;
            }
            wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } => {
                state.pointer_pos = (surface_x, surface_y);
            }
            wl_pointer::Event::Button {
                button,
                state: button_state,
                ..
            } => {
                if button_state == wayland_client::WEnum::Value(wl_pointer::ButtonState::Pressed) {
                    let (x, y) = state.pointer_pos;
                    state.dispatch_to_surface(Event::Click { x, y, button });
                }
            }
            wl_pointer::Event::Axis { axis, value, .. } => {
                let axis = match axis {
                    wayland_client::WEnum::Value(v) => v as u32,
                    wayland_client::WEnum::Unknown(v) => v,
                };
                state.dispatch_to_surface(Event::Scroll { axis, value });
            }
            _ => {}
        }
    }
}
