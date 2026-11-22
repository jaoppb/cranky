use super::super::command::SurfaceCommand;
use super::super::floating_bounds::to_i32;
use super::super::state::WaylandState;
use super::{bar_index, copy_surface_data};
use crate::shared::events::core::SurfaceKind;
use crate::shared::wayland::adapters::shm::{ShmBuffer, ShmSource};
use crate::shared::wayland::ports::DisplayServerError;
use wayland_client::QueueHandle;

/// The root module paints its own layer-surface buffer directly — no
/// subsurface, no parent (`SurfaceCommand::parent() == None`).
pub(super) fn handle_bar_surface(
    state: &mut WaylandState,
    queue_handle: &QueueHandle<WaylandState>,
    cmd: &SurfaceCommand,
) -> Result<(), DisplayServerError> {
    // handle_cmd routes a frame here only once wl_shm is bound and its monitor has a bar.
    let (Some(shm), Some(bar)) = (
        state.shm.clone(),
        bar_index(state, cmd).and_then(|index| state.bars.get_mut(index)),
    ) else {
        return Ok(());
    };
    let size = *cmd.buffer().size();

    if bar.shm_buffer.width() != size.width() || bar.shm_buffer.height() != size.height() {
        bar.shm_buffer = ShmBuffer::new(
            ShmSource::new(&shm, state.app_env.xdg_runtime_dir().as_path()),
            size,
            queue_handle,
        )
        .map_err(|error| DisplayServerError::Internal(error.to_string()))?;
    }

    copy_surface_data(&mut bar.shm_buffer, cmd.buffer().data());

    bar.surface.set_buffer_scale(bar.scale);
    bar.surface
        .attach(Some(bar.shm_buffer.current_buffer()), 0, 0);
    bar.surface
        .damage_buffer(0, 0, to_i32(size.width()), to_i32(size.height()));
    bar.surface.commit();
    bar.shm_buffer.swap_buffers();

    state.surface_to_id.insert(
        bar.surface.clone(),
        (cmd.module_id(), cmd.monitor_id().clone(), SurfaceKind::Bar),
    );

    Ok(())
}
