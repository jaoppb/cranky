use super::super::command::SurfaceCommand;
use super::super::state::WaylandState;
use super::subsurface::SubsurfaceFrame;
use crate::shared::wayland::ports::DisplayServerError;
use wayland_client::QueueHandle;

/// Creates or updates a module's own subsurface, parented to whichever
/// physical surface `parent` names — `bar.surface` for a module embedded in
/// the bar tree, or a popup/panel's own surface for one embedded via
/// `ui.module(name)` inside `ui.popup`/`ui.panel` (gap 4). Scale still comes
/// from the bar's own monitor entry regardless of parent: scale is a
/// per-monitor property, not a per-surface one.
pub(super) fn handle_module_surface(
    state: &mut WaylandState,
    queue_handle: &QueueHandle<WaylandState>,
    cmd: &SurfaceCommand,
) -> Result<(), DisplayServerError> {
    let Some(frame) = SubsurfaceFrame::for_command(state, queue_handle, cmd)? else {
        return Ok(());
    };
    // The parent surface not existing yet (a popup whose child rendered
    // before `ShowFloatingSurface` was processed) or not existing any more
    // (torn down after the child's last render) both mean: nothing to
    // parent against this frame. Drop it — the child resubmits on its own
    // next signal tick or bounds change.
    let Some((parent_surface, surface_kind)) = frame.parent_surface(state) else {
        return Ok(());
    };
    let Some(module_surface) = frame.module_surface(state, &parent_surface)? else {
        return Ok(());
    };
    frame.present(module_surface, &parent_surface)?;
    let ms_surface = module_surface.surface.clone();
    if let Some(kind) = surface_kind {
        state.surface_to_id.insert(ms_surface, frame.route(kind));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::handle_module_surface;
    use crate::shared::config::domain::PopupEmbed;
    use crate::shared::primitives::geometry::{Position, Size};
    use crate::shared::primitives::render::{ModuleFrame, PlacedBuffer};
    use crate::shared::primitives::{ModuleId, MonitorId};
    use crate::shared::wayland::adapters::wayland::command::SurfaceCommand;
    use crate::shared::wayland::adapters::wayland::tests::fixture::{
        Fake, lose_runtime_dir, solid,
    };
    use crate::shared::wayland::ports::DisplayServerError;
    use crate::shared::wayland::surface_parent::SurfaceParent;

    fn frame(size: Size) -> SurfaceCommand {
        let buffer = solid(size.width(), size.height(), [1; 4]);
        SurfaceCommand::new(
            ModuleFrame::new(
                ModuleId::new(2),
                MonitorId::new("DP-1"),
                PlacedBuffer::new(Position::new(0, 0), buffer),
            ),
            Some(SurfaceParent::Bar),
        )
    }

    /// A resubmission at a new size whose buffer cannot be allocated fails
    /// and keeps the module's old buffer.
    #[test]
    fn module_resize_without_memory_keeps_the_old_buffer() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        state.bars.push(fake.bar("DP-1", 1));
        let first = frame(Size::new(1, 1));
        handle_module_surface(&mut state, &fake.queue_handle, &first).expect("first frame");
        lose_runtime_dir(&mut state);
        let bigger = frame(Size::new(3, 2));
        let result = handle_module_surface(&mut state, &fake.queue_handle, &bigger);
        assert!(matches!(result, Err(DisplayServerError::Internal(_))));
        let module = &state.bars[0].module_surfaces[&ModuleId::new(2)];
        assert_eq!(module.size, Size::new(1, 1));
        assert_eq!(module.shm_buffer.width(), 1);
    }
}
