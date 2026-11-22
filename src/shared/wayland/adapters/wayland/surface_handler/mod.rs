mod bar;
mod composite;
mod module;

use super::command::SurfaceCommand;
use super::state::WaylandState;
use crate::shared::config::domain::PopupEmbed;
use crate::shared::wayland::adapters::shm::ShmBuffer;
use crate::shared::wayland::ports::DisplayServerError;
use crate::shared::wayland::surface_parent::SurfaceParent;
use bar::handle_bar_surface;
use composite::handle_composite_child;
use module::handle_module_surface;
use wayland_client::QueueHandle;

pub(super) fn copy_surface_data(shm_buffer: &mut ShmBuffer, src_data: &[u8]) {
    let data = shm_buffer.mmap_mut();
    let len = std::cmp::min(data.len(), src_data.len());
    if let Some(dest) = data.get_mut(..len)
        && let Some(src) = src_data.get(..len)
    {
        dest.copy_from_slice(src);
    }
}

pub(super) fn handle_cmd(
    state: &mut WaylandState,
    queue_handle: &QueueHandle<WaylandState>,
    cmd: &SurfaceCommand,
) -> Result<(), DisplayServerError> {
    if state.shm.is_none() || bar_index(state, cmd).is_none() {
        return Ok(());
    }

    let embed = state.hub.config_rx().borrow().popup().embed();
    match cmd.parent() {
        None => handle_bar_surface(state, queue_handle, cmd),
        Some(SurfaceParent::Floating(kind)) if embed == PopupEmbed::Composite => {
            handle_composite_child(state, kind, cmd);
            Ok(())
        }
        Some(_) => handle_module_surface(state, queue_handle, cmd),
    }
}

/// The index of the bar on `cmd`'s monitor, if that monitor has one.
fn bar_index(state: &WaylandState, cmd: &SurfaceCommand) -> Option<usize> {
    state
        .bars
        .iter()
        .position(|bar| bar.output_name == cmd.monitor_id().as_str())
}
