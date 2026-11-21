use super::dispatch_pointer::{handle_pointer_enter, handle_pointer_leave, handle_pointer_motion};
use super::dispatch_pointer_button::{handle_pointer_axis, handle_pointer_button};
use super::keyboard::KeyboardPresence;
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
        queue_handle: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities { capabilities } = event {
            let caps =
                wayland_client::protocol::wl_seat::Capability::from_bits(capabilities.into())
                    .unwrap_or(wayland_client::protocol::wl_seat::Capability::empty());
            if caps.contains(wayland_client::protocol::wl_seat::Capability::Pointer)
                && state.pointer.is_none()
            {
                state.pointer = Some(proxy.get_pointer(queue_handle, ()));
            }
            let presence = if caps.contains(wl_seat::Capability::Keyboard) {
                KeyboardPresence::Present
            } else {
                KeyboardPresence::Absent
            };
            state.keyboard.on_capability(proxy, presence, queue_handle);
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
            wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } => {
                handle_pointer_motion(state, surface_x, surface_y);
            }
            wl_pointer::Event::Button {
                button,
                state: btn_state,
                serial,
                ..
            } => {
                handle_pointer_button(state, button, btn_state, serial);
            }
            wl_pointer::Event::Axis { axis, value, .. } => handle_pointer_axis(state, axis, value),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::shared::config::domain::PopupEmbed;
    use crate::shared::wayland::adapters::wayland::keyboard::KeyboardInput;
    use crate::shared::wayland::adapters::wayland::tests::fixture::Fake;
    use wayland_client::protocol::wl_seat::Capability;

    const ESC_EVDEV: u32 = 1;

    #[test]
    fn keyboard_capability_binds_and_its_loss_forgets_the_keyboard() {
        let fake = Fake::new();
        let seat = fake.seat();
        let mut state = fake.state(PopupEmbed::Composite);
        state.keyboard = KeyboardInput::with_escape_code(ESC_EVDEV);
        fake.seat_capabilities(
            &mut state,
            &seat,
            Capability::Keyboard | Capability::Pointer,
        );
        assert!(state.pointer.is_some());
        assert!(state.keyboard.is_escape(ESC_EVDEV));
        fake.seat_capabilities(&mut state, &seat, Capability::empty());
        assert!(!state.keyboard.is_escape(ESC_EVDEV));
    }

    #[test]
    fn seat_without_keyboard_binds_none() {
        let fake = Fake::new();
        let seat = fake.seat();
        let mut state = fake.state(PopupEmbed::Composite);
        state.keyboard = KeyboardInput::with_escape_code(ESC_EVDEV);
        fake.seat_capabilities(&mut state, &seat, Capability::Pointer);
        // No keyboard was bound, so losing it has nothing to forget.
        fake.seat_capabilities(&mut state, &seat, Capability::empty());
        assert!(state.keyboard.is_escape(ESC_EVDEV));
    }
}
