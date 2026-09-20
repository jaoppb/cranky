use super::super::command::SurfaceCommand;
use super::super::state::WaylandState;
use super::super::types::ModuleSurface;
use super::copy_surface_data;
use crate::features::layout_engine::domain::FloatingKind;
use crate::shared::events::core::SurfaceKind;
use crate::shared::primitives::ModuleId;
use crate::shared::wayland::adapters::shm::ShmBuffer;
use crate::shared::wayland::ports::DisplayServerError;
use crate::shared::wayland::surface_parent::SurfaceParent;
use std::collections::HashMap;
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::protocol::wl_surface::WlSurface;

/// The surface a child's subsurface should be parented to, plus the
/// `SurfaceKind` to record for it in `surface_to_id` — `None` for a tooltip
/// child, which (like the tooltip's own surface in `setup_new_floating`)
/// gets no pointer routing today. `None` overall means the parent isn't
/// ready yet (a popup not yet shown, or already torn down).
fn resolve_parent_surface(
    state: &WaylandState,
    bar_index: usize,
    parent: &SurfaceParent,
) -> Option<(WlSurface, Option<SurfaceKind>)> {
    match parent {
        SurfaceParent::Bar => state
            .bars
            .get(bar_index)
            .map(|bar| (bar.surface.clone(), Some(SurfaceKind::Bar))),
        SurfaceParent::Floating(kind) => {
            let surface_kind = match kind {
                FloatingKind::Popup(_) => Some(SurfaceKind::Popup),
                FloatingKind::Panel(_) => Some(SurfaceKind::Panel),
                FloatingKind::Tooltip => None,
            };
            state
                .floating_surfaces
                .get(kind)
                .map(|floating| (floating.surface.clone(), surface_kind))
        }
    }
}

fn module_surfaces_mut<'a>(
    state: &'a mut WaylandState,
    bar_index: usize,
    parent: &SurfaceParent,
) -> Option<&'a mut HashMap<ModuleId, ModuleSurface>> {
    match parent {
        SurfaceParent::Bar => state
            .bars
            .get_mut(bar_index)
            .map(|bar| &mut bar.module_surfaces),
        SurfaceParent::Floating(kind) => state
            .floating_surfaces
            .get_mut(kind)
            .map(|floating| &mut floating.module_surfaces),
    }
}

/// Creates or updates a module's own subsurface, parented to whichever
/// physical surface `parent` names — `bar.surface` for a module embedded in
/// the bar tree, or a popup/panel's own surface for one embedded via
/// `ui.module(name)` inside `ui.popup`/`ui.panel` (gap 4). Scale still comes
/// from the bar's own monitor entry regardless of parent: scale is a
/// per-monitor property, not a per-surface one.
pub(super) fn handle_module_surface(
    state: &mut WaylandState,
    qh: &QueueHandle<WaylandState>,
    shm: &WlShm,
    bar_index: usize,
    parent: &SurfaceParent,
    cmd: &SurfaceCommand,
    (width, height, w_i32, h_i32): (u32, u32, i32, i32),
) -> Result<(), DisplayServerError> {
    let compositor = state
        .compositor
        .clone()
        .ok_or_else(|| DisplayServerError::Internal("Compositor not bound".to_string()))?;
    let subcompositor = state
        .subcompositor
        .clone()
        .ok_or_else(|| DisplayServerError::Internal("Subcompositor not bound".to_string()))?;
    let bar_scale = state
        .bars
        .get(bar_index)
        .map(|bar| bar.scale)
        .ok_or_else(|| DisplayServerError::Internal("Bar not found".to_string()))?;
    let xdg_runtime_dir = state.app_env.xdg_runtime_dir().as_path().clone();

    // The parent surface not existing yet (a popup whose child rendered
    // before `ShowFloatingSurface` was processed) or not existing any more
    // (torn down after the child's last render) both mean: nothing to
    // parent against this frame. Drop it — the child resubmits on its own
    // next signal tick or bounds change.
    let Some((parent_surface, surface_kind)) = resolve_parent_surface(state, bar_index, parent)
    else {
        return Ok(());
    };

    let mut new_surface_to_register = None;
    if let Some(module_surfaces) = module_surfaces_mut(state, bar_index, parent)
        && let std::collections::hash_map::Entry::Vacant(e) = module_surfaces.entry(cmd.module_id())
    {
        let surface = compositor.create_surface(qh, ());
        surface.set_buffer_scale(bar_scale);
        let subsurface = subcompositor.get_subsurface(&surface, &parent_surface, qh, ());
        subsurface.set_desync();

        let shm_buffer = ShmBuffer::new(shm, width, height, qh, xdg_runtime_dir.as_path())
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

    if let Some(surface) = new_surface_to_register
        && let Some(kind) = surface_kind
    {
        state
            .surface_to_id
            .insert(surface, (cmd.module_id(), cmd.monitor_id().clone(), kind));
    }

    let Some(ms) = module_surfaces_mut(state, bar_index, parent)
        .and_then(|surfaces| surfaces.get_mut(&cmd.module_id()))
    else {
        return Ok(());
    };

    if ms.size != *cmd.buffer().size() {
        ms.shm_buffer = ShmBuffer::new(shm, width, height, qh, xdg_runtime_dir.as_path())
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

    ms.surface.set_buffer_scale(bar_scale);
    ms.surface
        .attach(Some(ms.shm_buffer.current_buffer()), 0, 0);
    ms.surface.damage_buffer(0, 0, w_i32, h_i32);
    ms.surface.commit();
    parent_surface.commit();
    ms.shm_buffer.swap_buffers();
    let ms_surface = ms.surface.clone();

    if let Some(kind) = surface_kind {
        state.surface_to_id.insert(
            ms_surface,
            (cmd.module_id(), cmd.monitor_id().clone(), kind),
        );
    }

    Ok(())
}
