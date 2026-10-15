use super::super::command::SurfaceCommand;
use super::super::state::WaylandState;
use super::copy_surface_data;
use crate::shared::events::core::SurfaceKind;
use crate::shared::wayland::adapters::shm::ShmBuffer;
use crate::shared::wayland::ports::DisplayServerError;
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_shm::WlShm;

/// The root module paints its own layer-surface buffer directly — no
/// subsurface, no parent (`SurfaceCommand::parent() == None`).
pub(super) fn handle_bar_surface(
    state: &mut WaylandState,
    qh: &QueueHandle<WaylandState>,
    shm: &WlShm,
    bar_index: usize,
    cmd: &SurfaceCommand,
    (width, height, w_i32, h_i32): (u32, u32, i32, i32),
) -> Result<(), DisplayServerError> {
    let bar = state
        .bars
        .get_mut(bar_index)
        .ok_or_else(|| DisplayServerError::Internal("Bar not found".to_string()))?;

    if bar.shm_buffer.width() != width || bar.shm_buffer.height() != height {
        bar.shm_buffer = ShmBuffer::new(
            shm,
            width,
            height,
            qh,
            state.app_env.xdg_runtime_dir().as_path(),
        )
        .map_err(|e| DisplayServerError::Internal(e.to_string()))?;
    }

    copy_surface_data(&mut bar.shm_buffer, cmd.buffer().data());

    bar.surface.set_buffer_scale(bar.scale);
    bar.surface
        .attach(Some(bar.shm_buffer.current_buffer()), 0, 0);
    bar.surface.damage_buffer(0, 0, w_i32, h_i32);
    bar.surface.commit();
    bar.shm_buffer.swap_buffers();

    state.surface_to_id.insert(
        bar.surface.clone(),
        (cmd.module_id(), cmd.monitor_id().clone(), SurfaceKind::Bar),
    );

    Ok(())
}
