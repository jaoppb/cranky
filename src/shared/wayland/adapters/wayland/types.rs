use crate::shared::config::domain::MarginConfig;
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::ModuleId;
use crate::shared::wayland::adapters::shm::ShmBuffer;
use std::collections::HashMap;
use std::os::unix::io::{AsRawFd, RawFd};
use wayland_client::protocol::{
    wl_output::WlOutput, wl_subsurface::WlSubsurface, wl_surface::WlSurface,
};
use wayland_protocols::xdg::shell::client::{xdg_popup::XdgPopup, xdg_surface::XdgSurface};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::ZwlrLayerSurfaceV1;

pub(crate) struct WaylandFd(pub(crate) RawFd);

impl AsRawFd for WaylandFd {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

pub(crate) struct FloatingSurface {
    pub(crate) surface: WlSurface,
    pub(crate) xdg_surface: XdgSurface,
    pub(crate) xdg_popup: XdgPopup,
    pub(crate) shm_buffer: ShmBuffer,
    pub(crate) size: Size,
    pub(crate) reposition_token: u32,
    /// Modules embedded inside this floating surface's own content (gap 4)
    /// — each gets its own subsurface, parented here instead of to
    /// `bar.surface`. Dropped automatically (destroying every child
    /// subsurface) whenever this `FloatingSurface` itself is dropped, since
    /// field drop order runs after this type's own `Drop::drop` body.
    pub(crate) module_surfaces: HashMap<ModuleId, ModuleSurface>,
}

impl Drop for FloatingSurface {
    fn drop(&mut self) {
        self.xdg_popup.destroy();
        self.xdg_surface.destroy();
        self.surface.destroy();
    }
}

pub(crate) struct WaylandOutputInfo {
    pub(crate) global_id: u32,
    pub(crate) output: WlOutput,
    pub(crate) name: String,
    pub(crate) scale: i32,
}

pub(crate) struct WaylandBar {
    pub(crate) output_name: String,
    pub(crate) surface: WlSurface,
    pub(crate) layer_surface: ZwlrLayerSurfaceV1,
    pub(crate) shm_buffer: ShmBuffer,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) config_height: u32,
    pub(crate) config_margin: MarginConfig,
    pub(crate) scale: i32,
    pub(crate) module_surfaces: HashMap<ModuleId, ModuleSurface>,
    pub(crate) configured: bool,
}

pub(crate) struct ModuleSurface {
    pub(crate) surface: WlSurface,
    pub(crate) subsurface: WlSubsurface,
    pub(crate) shm_buffer: ShmBuffer,
    pub(crate) size: Size,
    pub(crate) x: i32,
    pub(crate) y: i32,
}

impl Drop for ModuleSurface {
    fn drop(&mut self) {
        self.subsurface.destroy();
        self.surface.destroy();
    }
}

impl Drop for WaylandBar {
    fn drop(&mut self) {
        self.module_surfaces.clear();
        self.layer_surface.destroy();
        self.surface.destroy();
    }
}
