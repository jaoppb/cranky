//! The fixture's module subsurfaces.

use super::Fake;
use crate::shared::primitives::geometry::Size;
use crate::shared::wayland::adapters::wayland::types::ModuleSurface;
use wayland_client::protocol::wl_surface::WlSurface;

impl Fake {
    /// A 1×1 module subsurface parented to `parent`, at its origin.
    pub(crate) fn module_surface(&self, parent: &WlSurface) -> ModuleSurface {
        let surface = self.surface();
        let subsurface =
            self.subcompositor
                .get_subsurface(&surface, parent, &self.queue_handle, ());
        ModuleSurface {
            surface,
            subsurface,
            shm_buffer: self.shm_buffer(Size::new(1, 1)),
            size: Size::new(1, 1),
            x: 0,
            y: 0,
        }
    }
}
