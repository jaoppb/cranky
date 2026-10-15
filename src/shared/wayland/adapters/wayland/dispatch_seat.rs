use super::dispatch_pointer::{handle_pointer_enter, handle_pointer_leave, handle_pointer_motion};
use super::dispatch_pointer_button::{handle_pointer_axis, handle_pointer_button};
use super::state::WaylandState;
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
