mod bar;
mod composite;
mod module;

use super::command::SurfaceCommand;
use super::state::WaylandState;
use crate::shared::wayland::adapters::shm::ShmBuffer;
use crate::shared::wayland::ports::DisplayServerError;
use crate::shared::config::domain::PopupEmbed;
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

pub(crate) fn handle_cmd(
    state: &mut WaylandState,
    qh: &QueueHandle<WaylandState>,
    cmd: &SurfaceCommand,
) -> Result<(), DisplayServerError> {
    let Some(shm) = state.shm.clone() else {
        return Ok(());
    };

    let Some(bar_index) = state
        .bars
        .iter()
        .position(|b| b.output_name == cmd.monitor_id().as_str())
    else {
        return Ok(());
    };

    let width = cmd.buffer().size().width();
    let height = cmd.buffer().size().height();
    let dims = (
        width,
        height,
        i32::try_from(width).unwrap_or(0),
        i32::try_from(height).unwrap_or(0),
    );

    let embed = state.hub.config_rx().borrow().popup().embed();
    match cmd.parent() {
        None => handle_bar_surface(state, qh, &shm, bar_index, cmd, dims),
        Some(SurfaceParent::Floating(kind)) if embed == PopupEmbed::Composite => {
            handle_composite_child(state, bar_index, kind, cmd);
            Ok(())
        }
        Some(parent) => handle_module_surface(state, qh, &shm, bar_index, parent, cmd, dims),
    }
}
