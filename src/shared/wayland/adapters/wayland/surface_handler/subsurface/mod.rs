//! One module frame on its own subsurface: what showing it takes, and the
//! creating, resizing, moving and committing of that subsurface.
mod parent;

use super::super::command::SurfaceCommand;
use super::super::input_region::{is_click_through, set_empty_input_region};
use super::super::state::{WaylandState, allocate_buffer};
use super::super::types::ModuleSurface;
use super::{attach_and_commit, bar_index, copy_surface_data};
use crate::shared::events::core::SurfaceKind;
use crate::shared::primitives::geometry::Size;
use crate::shared::wayland::adapters::shm::{ShmBuffer, ShmSource};
use crate::shared::wayland::composite::route::PointerTarget;
use crate::shared::wayland::ports::DisplayServerError;
use crate::shared::wayland::surface_parent::SurfaceParent;
use std::collections::hash_map::Entry;
use std::path::PathBuf;
use wayland_client::QueueHandle;
use wayland_client::protocol::{
    wl_compositor::WlCompositor, wl_shm::WlShm, wl_subcompositor::WlSubcompositor,
    wl_surface::WlSurface,
};

/// One module frame for a subsurface, with what showing it takes: its
/// parent, its bar and that bar's scale, the globals and the queue.
pub(super) struct SubsurfaceFrame<'a> {
    cmd: &'a SurfaceCommand,
    parent: &'a SurfaceParent,
    bar_index: usize,
    bar_scale: i32,
    compositor: WlCompositor,
    subcompositor: WlSubcompositor,
    shm: WlShm,
    xdg_runtime_dir: PathBuf,
    queue_handle: &'a QueueHandle<WaylandState>,
}

impl<'a> SubsurfaceFrame<'a> {
    /// `cmd` bound to what showing it takes, or `None` while it has no
    /// parent, its monitor no bar, or `wl_shm` is not bound.
    pub(super) fn for_command(
        state: &WaylandState,
        queue_handle: &'a QueueHandle<WaylandState>,
        cmd: &'a SurfaceCommand,
    ) -> Result<Option<Self>, DisplayServerError> {
        // handle_cmd routes a frame here only with a parent, once wl_shm is bound and its monitor
        // has a bar.
        let (Some(parent), Some(bar_index), Some(shm)) =
            (cmd.parent(), bar_index(state, cmd), state.shm.clone())
        else {
            return Ok(None);
        };
        let Some(bar_scale) = state.bars.get(bar_index).map(|bar| bar.scale) else {
            return Ok(None);
        };
        let compositor = owned_global(state.compositor.as_ref(), "Compositor")?;
        let subcompositor = owned_global(state.subcompositor.as_ref(), "Subcompositor")?;
        let xdg_runtime_dir = state.app_env.xdg_runtime_dir().as_path().clone();
        Ok(Some(Self {
            cmd,
            parent,
            bar_index,
            bar_scale,
            compositor,
            subcompositor,
            shm,
            xdg_runtime_dir,
            queue_handle,
        }))
    }

    const fn size(&self) -> Size {
        *self.cmd.buffer().size()
    }

    /// The pointer route to the frame's module, as a surface of `kind`.
    pub(super) fn route(&self, kind: SurfaceKind) -> PointerTarget {
        (self.cmd.module_id(), self.cmd.monitor_id().clone(), kind)
    }

    /// A buffer of the frame's size.
    fn buffer(&self) -> Result<ShmBuffer, DisplayServerError> {
        let source = ShmSource::new(&self.shm, &self.xdg_runtime_dir);
        allocate_buffer(source, self.size(), self.queue_handle)
    }

    /// The frame's module surface in its parent, first created under
    /// `parent_surface` when the parent has none; `None` while the parent
    /// does not exist.
    pub(super) fn module_surface<'s>(
        &self,
        state: &'s mut WaylandState,
        parent_surface: &WlSurface,
    ) -> Result<Option<&'s mut ModuleSurface>, DisplayServerError> {
        let Some(module_surfaces) = self.module_surfaces_mut(state) else {
            return Ok(None);
        };
        let module_surface = match module_surfaces.entry(self.cmd.module_id()) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(self.create_module_surface(parent_surface)?),
        };
        Ok(Some(module_surface))
    }

    /// A desynchronized subsurface at the bar's scale under
    /// `parent_surface`, click-through inside a tooltip, with a buffer of the
    /// frame's size, at the parent's origin.
    fn create_module_surface(
        &self,
        parent_surface: &WlSurface,
    ) -> Result<ModuleSurface, DisplayServerError> {
        let surface = self.compositor.create_surface(self.queue_handle, ());
        surface.set_buffer_scale(self.bar_scale);
        let subsurface =
            self.subcompositor
                .get_subsurface(&surface, parent_surface, self.queue_handle, ());
        subsurface.set_desync();
        if matches!(self.parent, SurfaceParent::Floating(kind) if is_click_through(kind)) {
            set_empty_input_region(&self.compositor, &surface, self.queue_handle);
        }
        let shm_buffer = self.buffer()?;
        Ok(ModuleSurface {
            surface,
            subsurface,
            shm_buffer,
            size: self.size(),
            x: 0,
            y: 0,
        })
    }

    /// Shows the frame on `module_surface`, resized and moved to it, then
    /// commits `parent_surface`.
    pub(super) fn present(
        &self,
        module_surface: &mut ModuleSurface,
        parent_surface: &WlSurface,
    ) -> Result<(), DisplayServerError> {
        if module_surface.size != self.size() {
            module_surface.shm_buffer = self.buffer()?;
            module_surface.size = self.size();
        }
        module_surface.move_to(self.cmd.position());
        copy_surface_data(&mut module_surface.shm_buffer, self.cmd.buffer().data());
        attach_and_commit(
            &module_surface.surface,
            &module_surface.shm_buffer,
            self.bar_scale,
        );
        parent_surface.commit();
        module_surface.shm_buffer.swap_buffers();
        Ok(())
    }
}

/// A clone of `global`, or the internal error naming it as not bound.
fn owned_global<T: Clone>(global: Option<&T>, name: &str) -> Result<T, DisplayServerError> {
    global
        .cloned()
        .ok_or_else(|| DisplayServerError::Internal(format!("{name} not bound")))
}
