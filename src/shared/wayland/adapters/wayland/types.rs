use super::floating_bounds::ChildPlacement;
use crate::shared::config::domain::MarginConfig;
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::ModuleId;
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::wayland::adapters::shm::ShmBuffer;
use crate::shared::wayland::composite::CompositeLayers;
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
    /// The floating surface this one is nested inside (decision 7), fixed
    /// at creation — `None` for one owned by a module embedded directly in
    /// the bar tree. Read back as the parent link when walking ancestor or
    /// descendant chains for exclusivity and teardown ordering.
    pub(crate) parent: Option<crate::features::layout_engine::domain::FloatingKind>,
    /// The owner's last rendered frame, kept so a child update can re-blend
    /// without the owner re-rendering.
    pub(crate) base: RenderBuffer,
    /// Embedded children's pixels, blended over `base` (composite mode).
    pub(crate) layers: CompositeLayers,
    /// Whether the first `xdg_surface.configure` arrived; until then nothing
    /// may be attached or committed.
    pub(crate) configure: ConfigureGate,
    /// Buffer scale the surface was last placed with (its anchor's bar scale).
    pub(crate) scale: i32,
}

/// Tracks a floating surface's first `xdg_surface.configure`.
#[derive(Debug, Default)]
pub(crate) struct ConfigureGate {
    configured: bool,
}

impl ConfigureGate {
    pub(crate) const fn is_configured(&self) -> bool {
        self.configured
    }

    pub(crate) const fn mark_configured(&mut self) {
        self.configured = true;
    }
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

impl ModuleSurface {
    pub(super) const fn placement(&self) -> ChildPlacement {
        ChildPlacement::new(self.x, self.y, self.size)
    }
}

impl FloatingSurface {
    /// Where `module_id` is embedded in this surface: a composited layer
    /// first, else an old-style subsurface.
    pub(super) fn child_placement(&self, module_id: ModuleId) -> Option<ChildPlacement> {
        self.layers
            .get(module_id)
            .map(|l| ChildPlacement::new(l.origin().x(), l.origin().y(), *l.buffer().size()))
            .or_else(|| {
                self.module_surfaces
                    .get(&module_id)
                    .map(ModuleSurface::placement)
            })
    }
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
