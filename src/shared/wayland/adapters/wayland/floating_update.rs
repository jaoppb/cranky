use super::floating_anchor::AnchorInfo;
use super::floating_composite::repaint;
use super::floating_render::create_positioner;
use super::floating_setup::FloatingPlacement;
use super::state::WaylandState;
use super::types::FloatingSurface;
use crate::features::layout_engine::domain::FloatingKind;
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::wayland::adapters::shm::ShmBuffer;
use crate::shared::wayland::ports::DisplayServerError;
use wayland_client::QueueHandle;

pub(super) fn commit_surface(
    floating: &mut FloatingSurface,
    bar_scale: i32,
    w_i32: i32,
    h_i32: i32,
) {
    floating.surface.set_buffer_scale(bar_scale);
    floating.surface.attach(Some(floating.shm_buffer.current_buffer()), 0, 0);
    floating.surface.damage_buffer(0, 0, w_i32, h_i32);
    floating.surface.commit();
    floating.shm_buffer.swap_buffers();
}

pub(super) fn update_existing_floating(
    state: &mut WaylandState,
    qh: &QueueHandle<WaylandState>,
    kind: &FloatingKind,
    buffer: &RenderBuffer,
    anchor_info: &AnchorInfo,
    placement: &FloatingPlacement,
) -> Result<(), DisplayServerError> {
    let Some(floating) = state.floating_surfaces.get_mut(kind) else {
        return Ok(());
    };
    floating.scale = anchor_info.bar_scale;

    if floating.size == placement.size {
        floating.base = buffer.clone();
        repaint(floating, anchor_info.bar_scale);
        return Ok(());
    }

    let shm = state.shm.as_ref().ok_or_else(|| DisplayServerError::ConnectionFailed {
        reason: "SHM not bound".to_string(),
    })?;
    let new_shm_buffer = ShmBuffer::new(
        shm,
        placement.size.width(),
        placement.size.height(),
        qh,
        state.app_env.xdg_runtime_dir().as_path(),
    )
    .map_err(|e| DisplayServerError::Internal(e.to_string()))?;

    let xdg_wm_base = state.xdg_wm_base.as_ref().ok_or_else(|| DisplayServerError::ConnectionFailed {
        reason: "XDG WM Base not bound".to_string(),
    })?;
    let positioner = create_positioner(
        xdg_wm_base,
        qh,
        placement.text_w,
        placement.text_h,
        anchor_info,
        placement.offset,
    );
    floating.reposition_token = floating.reposition_token.wrapping_add(1);
    floating.xdg_popup.reposition(&positioner, floating.reposition_token);
    positioner.destroy();
    // Blend into the new buffer, not the old one, before committing it.
    floating.shm_buffer = new_shm_buffer;
    floating.size = placement.size;
    // Only now: `base` must keep the shm buffer's size if any step above fails.
    floating.base = buffer.clone();
    repaint(floating, anchor_info.bar_scale);
    Ok(())
}
