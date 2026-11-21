//! The fixture's bar events: the layer surface's `configure`.

use super::Fake;
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

    /// Delivers a layer-surface `configure` of `width`×`height` to the bar at
    /// `index` through the real dispatch handler.
    pub(crate) fn configure_bar(
        &self,
        state: &mut WaylandState,
        index: usize,
        width: u32,
        height: u32,
    ) {
        let layer_surface = state
            .bars
            .get(index)
            .map(|bar| bar.layer_surface.clone())
            .expect("bar");
        <WaylandState as Dispatch<ZwlrLayerSurfaceV1, ()>>::event(
            state,
            &layer_surface,
            LayerSurfaceEvent::Configure {
                serial: 1,
                width,
                height,
            },
            &(),
            &self.connection,
            &self.queue_handle,
        );
    }
}
