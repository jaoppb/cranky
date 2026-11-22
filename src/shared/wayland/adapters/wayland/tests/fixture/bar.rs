//! The fixture's bar events: the layer surface's `configure`.

use super::Fake;
use crate::shared::primitives::geometry::Size;
use crate::shared::wayland::adapters::wayland::state::WaylandState;
use wayland_client::{Connection, Dispatch};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::{
    Event as LayerSurfaceEvent, ZwlrLayerSurfaceV1,
};

impl Fake {
    /// The client connection, for code under test that flushes it.
    pub(crate) const fn connection(&self) -> &Connection {
        &self.connection
    }

    /// Delivers a layer-surface `configure` of `size` to the first bar
    /// through the real dispatch handler.
    pub(crate) fn configure_bar(&self, state: &mut WaylandState, size: Size) {
        let layer_surface = state
            .bars
            .first()
            .map(|bar| bar.layer_surface.clone())
            .expect("bar");
        <WaylandState as Dispatch<ZwlrLayerSurfaceV1, ()>>::event(
            state,
            &layer_surface,
            LayerSurfaceEvent::Configure {
                serial: 1,
                width: size.width(),
                height: size.height(),
            },
            &(),
            &self.connection,
            &self.queue_handle,
        );
    }
}
