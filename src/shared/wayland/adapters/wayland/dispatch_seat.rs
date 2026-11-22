use super::dispatch_pointer::{handle_pointer_enter, handle_pointer_leave, handle_pointer_motion};
use super::dispatch_pointer_button::{
    handle_pointer_axis, handle_pointer_button, record_button_serial,
};
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
            if caps.contains(wl_seat::Capability::Keyboard) {
                state.keyboard.bind(proxy, queue_handle);
            } else {
                state.keyboard.release();
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
            } => handle_pointer_enter(state, &surface, (surface_x, surface_y)),
            wl_pointer::Event::Leave { .. } => handle_pointer_leave(state),
            wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } => {
                handle_pointer_motion(state, (surface_x, surface_y));
            }
            wl_pointer::Event::Button {
                button,
                state: btn_state,
                serial,
                ..
            } => {
                record_button_serial(state, serial);
                handle_pointer_button(state, button, btn_state);
            }
            wl_pointer::Event::Axis { axis, value, .. } => handle_pointer_axis(state, axis, value),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::shared::config::domain::PopupEmbed;
    use crate::shared::events::core::PointerEvent;
    use crate::shared::events::core::{PointerSerial, ScrollAxis, ScrollDelta, SurfaceKind};
    use crate::shared::wayland::adapters::wayland::keyboard::KeyboardInput;
    use crate::shared::wayland::adapters::wayland::state::WaylandState;
    use crate::shared::wayland::adapters::wayland::tests::fixture::Fake;
    use crate::shared::wayland::adapters::wayland::tests::fixture::pointer::{
        LEFT, PRESSED, RELEASED, child, clicks, drain, enter, leave, motion, popup,
    };
    use wayland_client::protocol::wl_pointer::{self, Axis, WlPointer};
    use wayland_client::protocol::wl_seat::Capability;
    use wayland_client::{Dispatch, WEnum};

    const ESC_EVDEV: u32 = 1;

    #[test]
    fn keyboard_capability_binds_and_its_loss_forgets_the_keyboard() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        state.seat = Some(fake.seat());
        state.keyboard = KeyboardInput::with_escape_code(ESC_EVDEV);
        fake.seat_capabilities(&mut state, Capability::Keyboard | Capability::Pointer);
        assert!(state.pointer.is_some());
        assert!(state.keyboard.is_escape(ESC_EVDEV));
        fake.seat_capabilities(&mut state, Capability::empty());
        assert!(!state.keyboard.is_escape(ESC_EVDEV));
    }

    #[test]
    fn seat_without_keyboard_binds_none() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        state.seat = Some(fake.seat());
        state.keyboard = KeyboardInput::with_escape_code(ESC_EVDEV);
        fake.seat_capabilities(&mut state, Capability::Pointer);
        // No keyboard was bound, so losing it has nothing to forget.
        fake.seat_capabilities(&mut state, Capability::empty());
        assert!(state.keyboard.is_escape(ESC_EVDEV));
    }

    /// Delivers `event` through the real `wl_pointer` dispatch handler.
    fn deliver(fake: &Fake, state: &mut WaylandState, event: wl_pointer::Event) {
        let pointer = fake.seat().get_pointer(&fake.queue_handle, ());
        <WaylandState as Dispatch<WlPointer, ()>>::event(
            state,
            &pointer,
            event,
            &(),
            fake.connection(),
            &fake.queue_handle,
        );
    }

    #[test]
    fn pointer_events_reach_the_module_under_the_pointer() {
        let fake = Fake::new();
        let (mut state, surface) = popup(&fake, 1);
        let mut rx = state.hub.pointer_rx();
        let button = |serial, state| wl_pointer::Event::Button {
            serial,
            time: 0,
            button: LEFT,
            state,
        };
        deliver(
            &fake,
            &mut state,
            wl_pointer::Event::Enter {
                serial: 1,
                surface: surface.clone(),
                surface_x: 11.0,
                surface_y: 3.0,
            },
        );
        let motion_to = |surface_x| wl_pointer::Event::Motion {
            time: 0,
            surface_x,
            surface_y: 3.0,
        };
        deliver(&fake, &mut state, motion_to(12.0));
        deliver(&fake, &mut state, button(7, PRESSED));
        deliver(&fake, &mut state, button(8, RELEASED));
        let axis = wl_pointer::Event::Axis {
            time: 0,
            axis: WEnum::Value(Axis::VerticalScroll),
            value: 2.5,
        };
        deliver(&fake, &mut state, axis);
        deliver(&fake, &mut state, wl_pointer::Event::Frame);
        let left = wl_pointer::Event::Leave { serial: 9, surface };
        deliver(&fake, &mut state, left);

        let mut expected = vec![
            child(enter(SurfaceKind::Bar)),
            child(motion(SurfaceKind::Bar, 1, 1)),
            child(motion(SurfaceKind::Bar, 2, 1)),
        ];
        expected.extend(clicks(SurfaceKind::Bar, 2, 1).map(child));
        expected.push(child(PointerEvent::Scroll {
            surface: SurfaceKind::Bar,
            axis: ScrollAxis::Vertical,
            amount: ScrollDelta::new(2.5),
        }));
        expected.push(child(leave(SurfaceKind::Bar)));
        assert_eq!(drain(&mut rx), expected);
        assert_eq!(state.last_button_serial, Some(PointerSerial::new(8)));
        assert!(state.pointer_surface.is_none());
    }
}
