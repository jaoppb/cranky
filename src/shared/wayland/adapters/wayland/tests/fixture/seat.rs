//! The fixture's seat: a bound `wl_seat`.

use super::Fake;
use wayland_client::protocol::wl_seat::WlSeat;

impl Fake {
    /// A `wl_seat` (version 7), to bind input devices from.
    pub(crate) fn seat(&self) -> WlSeat {
        let registry = self
            .connection
            .display()
            .get_registry(&self.queue_handle, ());
        registry.bind(6, 7, &self.queue_handle, ())
    }
}
