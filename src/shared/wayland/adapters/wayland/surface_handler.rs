#![allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation
)]

use super::command::SurfaceCommand;
use super::state::WaylandState;
use super::types::ModuleSurface;
use crate::shared::events::core::SurfaceKind;
use crate::shared::wayland::adapters::shm::ShmBuffer;
use crate::shared::wayland::ports::DisplayServerError;
use wayland_client::QueueHandle;

pub(crate) fn handle_cmd(
    state: &mut WaylandState,
    qh: &QueueHandle<WaylandState>,
    cmd: SurfaceCommand,
) -> Result<(), DisplayServerError> {
    let Some(compositor) = state.compositor.as_ref() else {
        return Ok(());
    };
    let Some(subcompositor) = state.subcompositor.as_ref() else {
        return Ok(());
    };
    let Some(shm) = state.shm.as_ref() else {
        return Ok(());
    };

    let Some(bar) = state
        .bars
        .iter_mut()
        .find(|b| b.output_name == cmd.monitor_id().as_str())
    else {
        return Ok(());
    };

    let width = cmd.buffer().size().width();
    let height = cmd.buffer().size().height();
    let src_data = cmd.buffer().data();

    if cmd.parent_id().is_none() {
        if bar.shm_buffer.width() != width || bar.shm_buffer.height() != height {
            bar.shm_buffer = ShmBuffer::new(
                shm,
                width,
                height,
                qh,
                state.app_env.xdg_runtime_dir().as_path(),
            )
            .expect("Failed to recreate SHM buffer for root bar");
        }

        let data = bar.shm_buffer.mmap_mut();
        let len = std::cmp::min(data.len(), src_data.len());
        data[..len].copy_from_slice(&src_data[..len]);

        bar.surface.set_buffer_scale(bar.scale);
        bar.surface
            .attach(Some(bar.shm_buffer.current_buffer()), 0, 0);
        bar.surface.damage_buffer(0, 0, width as i32, height as i32);
        bar.surface.commit();
        bar.shm_buffer.swap_buffers();

        state.surface_to_id.insert(
            bar.surface.clone(),
            (cmd.module_id(), cmd.monitor_id().clone(), SurfaceKind::Bar),
        );
    } else {
        let mut new_surface_to_register = None;
        let ms = bar
            .module_surfaces
            .entry(cmd.module_id())
            .or_insert_with(|| {
                let surface = compositor.create_surface(qh, ());
                surface.set_buffer_scale(bar.scale);
                let subsurface = subcompositor.get_subsurface(&surface, &bar.surface, qh, ());
                subsurface.set_desync();

                let shm_buffer = ShmBuffer::new(
                    shm,
                    width,
                    height,
                    qh,
                    state.app_env.xdg_runtime_dir().as_path(),
                )
                .expect("Failed to create SHM buffer");

                new_surface_to_register = Some(surface.clone());

                ModuleSurface {
                    surface,
                    subsurface,
                    shm_buffer,
                    size: *cmd.buffer().size(),
                    x: 0,
                    y: 0,
                }
            });

        if let Some(surface) = new_surface_to_register {
            state
                .surface_to_id
                .insert(surface, (cmd.module_id(), cmd.monitor_id().clone(), SurfaceKind::Bar));
        }

        if ms.size != *cmd.buffer().size() {
            ms.shm_buffer = ShmBuffer::new(
                shm,
                width,
                height,
                qh,
                state.app_env.xdg_runtime_dir().as_path(),
            )
            .expect("Failed to recreate SHM buffer for resize");
            ms.size = *cmd.buffer().size();
        }

        if ms.x != cmd.position().x() || ms.y != cmd.position().y() {
            ms.subsurface
                .set_position(cmd.position().x(), cmd.position().y());
            ms.x = cmd.position().x();
            ms.y = cmd.position().y();
        }

        let data = ms.shm_buffer.mmap_mut();
        let len = std::cmp::min(data.len(), src_data.len());
        data[..len].copy_from_slice(&src_data[..len]);

        ms.surface.set_buffer_scale(bar.scale);
        ms.surface
            .attach(Some(ms.shm_buffer.current_buffer()), 0, 0);
        ms.surface.damage_buffer(0, 0, width as i32, height as i32);
        ms.surface.commit();

        bar.surface.commit();

        ms.shm_buffer.swap_buffers();

        state.surface_to_id.insert(
            ms.surface.clone(),
            (cmd.module_id(), cmd.monitor_id().clone(), SurfaceKind::Bar),
        );
    }

    Ok(())
}
