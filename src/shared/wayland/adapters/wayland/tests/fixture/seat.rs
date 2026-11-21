//! The fixture's seat: a bound `wl_seat` and its capability events.

use super::Fake;
use crate::shared::wayland::adapters::wayland::state::WaylandState;
use wayland_client::protocol::wl_seat::{Capability as SeatCapability, Event as SeatEvent, WlSeat};
use wayland_client::{Dispatch, WEnum};

impl Fake {
    /// A `wl_seat` (version 7), to bind input devices from.
    pub(crate) fn seat(&self) -> WlSeat {
        let registry = self
            .connection
            .display()
            .get_registry(&self.queue_handle, ());
        registry.bind(6, 7, &self.queue_handle, ())
    }

    /// Delivers a `wl_seat.capabilities` event through the real dispatch
    /// handler.
    pub(crate) fn seat_capabilities(
        &self,
        state: &mut WaylandState,
        seat: &WlSeat,
        capabilities: SeatCapability,
    ) {
        <WaylandState as Dispatch<WlSeat, ()>>::event(
            state,
            seat,
            SeatEvent::Capabilities {
                capabilities: WEnum::Value(capabilities),
            },
            &(),
            &self.connection,
            &self.queue_handle,
        );
    }
}
