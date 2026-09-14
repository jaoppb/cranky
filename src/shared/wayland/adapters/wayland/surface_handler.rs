use super::command::SurfaceCommand;
use super::state::WaylandState;
use super::types::ModuleSurface;
use crate::shared::events::core::SurfaceKind;
use crate::shared::wayland::adapters::shm::ShmBuffer;
use crate::shared::wayland::ports::DisplayServerError;
use wayland_client::protocol::wl_shm::WlShm;
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

fn handle_bar_surface(
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

fn handle_module_surface(
    state: &mut WaylandState,
    qh: &QueueHandle<WaylandState>,
    shm: &WlShm,
    bar_index: usize,
    cmd: &SurfaceCommand,
    (width, height, w_i32, h_i32): (u32, u32, i32, i32),
) -> Result<(), DisplayServerError> {
    let compositor = state
        .compositor
        .as_ref()
        .ok_or_else(|| DisplayServerError::Internal("Compositor not bound".to_string()))?;
    let subcompositor = state
        .subcompositor
        .as_ref()
        .ok_or_else(|| DisplayServerError::Internal("Subcompositor not bound".to_string()))?;

    let bar = state
        .bars
        .get_mut(bar_index)
        .ok_or_else(|| DisplayServerError::Internal("Bar not found".to_string()))?;

    let bar_surface = bar.surface.clone();
    let bar_scale = bar.scale;
    let mut new_surface_to_register = None;

    if let std::collections::hash_map::Entry::Vacant(e) =
        bar.module_surfaces.entry(cmd.module_id())
    {
        let surface = compositor.create_surface(qh, ());
        surface.set_buffer_scale(bar_scale);
        let subsurface = subcompositor.get_subsurface(&surface, &bar_surface, qh, ());
        subsurface.set_desync();

        let shm_buffer = ShmBuffer::new(
            shm,
            width,
            height,
            qh,
            state.app_env.xdg_runtime_dir().as_path(),
        )
        .map_err(|err| DisplayServerError::Internal(err.to_string()))?;

        new_surface_to_register = Some(surface.clone());
        e.insert(ModuleSurface {
            surface,
            subsurface,
            shm_buffer,
            size: *cmd.buffer().size(),
            x: 0,
            y: 0,
        });
    }

    if let Some(surface) = new_surface_to_register {
        state
            .surface_to_id
            .insert(surface, (cmd.module_id(), cmd.monitor_id().clone(), SurfaceKind::Bar));
    }

    let bar = state
        .bars
        .get_mut(bar_index)
        .ok_or_else(|| DisplayServerError::Internal("Bar not found".to_string()))?;
    let ms = bar
        .module_surfaces
        .get_mut(&cmd.module_id())
        .ok_or_else(|| DisplayServerError::Internal("Module surface missing".to_string()))?;

    if ms.size != *cmd.buffer().size() {
        ms.shm_buffer = ShmBuffer::new(
            shm,
            width,
            height,
            qh,
            state.app_env.xdg_runtime_dir().as_path(),
        )
        .map_err(|e| DisplayServerError::Internal(e.to_string()))?;
        ms.size = *cmd.buffer().size();
    }

    if ms.x != cmd.position().x() || ms.y != cmd.position().y() {
        ms.subsurface
            .set_position(cmd.position().x(), cmd.position().y());
        ms.x = cmd.position().x();
        ms.y = cmd.position().y();
    }

    copy_surface_data(&mut ms.shm_buffer, cmd.buffer().data());

    ms.surface.set_buffer_scale(bar.scale);
    ms.surface
        .attach(Some(ms.shm_buffer.current_buffer()), 0, 0);
    ms.surface.damage_buffer(0, 0, w_i32, h_i32);
    ms.surface.commit();
    bar.surface.commit();
    ms.shm_buffer.swap_buffers();

    state.surface_to_id.insert(
        ms.surface.clone(),
        (cmd.module_id(), cmd.monitor_id().clone(), SurfaceKind::Bar),
    );

    Ok(())
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

    if cmd.parent_id().is_none() {
        handle_bar_surface(state, qh, &shm, bar_index, cmd, dims)
    } else {
        handle_module_surface(state, qh, &shm, bar_index, cmd, dims)
    }
}
