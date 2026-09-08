use super::floating_anchor::AnchorInfo;
use super::floating_render::{create_positioner, render_to_buffer, FloatingRenderParams};
use super::floating_surface::FloatingRenderContext;
use super::state::WaylandState;
use super::types::FloatingSurface;
use crate::features::layout_engine::domain::{FloatingKind, StyledNode};
use crate::shared::events::core::SurfaceKind;
use crate::shared::primitives::geometry::Size;
use crate::shared::wayland::adapters::shm::ShmBuffer;
use crate::shared::wayland::ports::DisplayServerError;
use wayland_client::QueueHandle;
use wayland_protocols::xdg::shell::client::xdg_positioner::XdgPositioner;

struct NewFloatingInfo {
    shm_buffer: ShmBuffer,
    size: Size,
    positioner: XdgPositioner,
}

fn commit_surface(floating: &mut FloatingSurface, bar_scale: i32, w_i32: i32, h_i32: i32) {
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
    layout: StyledNode,
    anchor_info: &AnchorInfo,
    ctx: &FloatingRenderContext<'_>,
) -> Result<(), DisplayServerError> {
    let Some(floating) = state.floating_surfaces.get_mut(kind) else {
        return Ok(());
    };
    let w_i32 = i32::try_from(ctx.width).unwrap_or(0);
    let h_i32 = i32::try_from(ctx.height).unwrap_or(0);

    if floating.size == ctx.size {
        render_to_buffer(FloatingRenderParams {
            shm_buffer: &mut floating.shm_buffer,
            render_node: ctx.render_node,
            font_system: &mut state.font_system,
            swash_cache: &mut state.swash_cache,
            width: ctx.width,
            height: ctx.height,
            scale: ctx.scale,
            font_family: ctx.font_family.clone(),
            font_size: ctx.font_size,
        });
        floating.layout = layout;
        commit_surface(floating, anchor_info.bar_scale, w_i32, h_i32);
        return Ok(());
    }

    let shm = state.shm.as_ref().ok_or_else(|| DisplayServerError::ConnectionFailed {
        reason: "SHM not bound".to_string(),
    })?;
    let mut new_shm_buffer = ShmBuffer::new(shm, ctx.width, ctx.height, qh, state.app_env.xdg_runtime_dir().as_path())
        .map_err(|e| DisplayServerError::Internal(e.to_string()))?;
    render_to_buffer(FloatingRenderParams {
        shm_buffer: &mut new_shm_buffer,
        render_node: ctx.render_node,
        font_system: &mut state.font_system,
        swash_cache: &mut state.swash_cache,
        width: ctx.width,
        height: ctx.height,
        scale: ctx.scale,
        font_family: ctx.font_family.clone(),
        font_size: ctx.font_size,
    });

    let xdg_wm_base = state.xdg_wm_base.as_ref().ok_or_else(|| DisplayServerError::ConnectionFailed {
        reason: "XDG WM Base not bound".to_string(),
    })?;
    let positioner = create_positioner(xdg_wm_base, qh, ctx.text_w, ctx.text_h, anchor_info, ctx.effective_offset);
    floating.reposition_token = floating.reposition_token.wrapping_add(1);
    floating.xdg_popup.reposition(&positioner, floating.reposition_token);
    positioner.destroy();
    floating.shm_buffer = new_shm_buffer;
    floating.size = ctx.size;
    floating.layout = layout;
    commit_surface(floating, anchor_info.bar_scale, w_i32, h_i32);
    Ok(())
}

pub(super) fn create_new_floating(
    state: &mut WaylandState,
    qh: &QueueHandle<WaylandState>,
    kind: FloatingKind,
    layout: StyledNode,
    anchor_info: &AnchorInfo,
    ctx: &FloatingRenderContext<'_>,
) -> Result<(), DisplayServerError> {
    let shm = state.shm.as_ref().ok_or_else(|| DisplayServerError::ConnectionFailed {
        reason: "SHM not bound".to_string(),
    })?;
    let mut shm_buffer = ShmBuffer::new(shm, ctx.width, ctx.height, qh, state.app_env.xdg_runtime_dir().as_path())
        .map_err(|e| DisplayServerError::Internal(e.to_string()))?;
    render_to_buffer(FloatingRenderParams {
        shm_buffer: &mut shm_buffer,
        render_node: ctx.render_node,
        font_system: &mut state.font_system,
        swash_cache: &mut state.swash_cache,
        width: ctx.width,
        height: ctx.height,
        scale: ctx.scale,
        font_family: ctx.font_family.clone(),
        font_size: ctx.font_size,
    });

    let xdg_wm_base = state.xdg_wm_base.as_ref().ok_or_else(|| DisplayServerError::ConnectionFailed {
        reason: "XDG WM Base not bound".to_string(),
    })?;
    let positioner = create_positioner(xdg_wm_base, qh, ctx.text_w, ctx.text_h, anchor_info, ctx.effective_offset);

    setup_new_floating(
        state,
        qh,
        kind,
        layout,
        anchor_info,
        NewFloatingInfo {
            shm_buffer,
            size: ctx.size,
            positioner,
        },
    )
}

fn setup_new_floating(
    state: &mut WaylandState,
    qh: &QueueHandle<WaylandState>,
    kind: FloatingKind,
    layout: StyledNode,
    anchor_info: &AnchorInfo,
    info: NewFloatingInfo,
) -> Result<(), DisplayServerError> {
    let xdg_wm_base = state.xdg_wm_base.as_ref().ok_or_else(|| DisplayServerError::ConnectionFailed {
        reason: "XDG WM Base not bound".to_string(),
    })?;
    let compositor = state.compositor.as_ref().ok_or_else(|| DisplayServerError::ConnectionFailed {
        reason: "Compositor not bound".to_string(),
    })?;
    let surface = compositor.create_surface(qh, ());
    surface.set_buffer_scale(anchor_info.bar_scale);
    let xdg_surface = xdg_wm_base.get_xdg_surface(&surface, qh, ());
    let xdg_popup = xdg_surface.get_popup(None, &info.positioner, qh, ());

    if let FloatingKind::Popup(_) = &kind
        && let (Some(seat), Some(serial)) = (state.seat.as_ref(), state.last_button_serial)
    {
        xdg_popup.grab(seat, serial.value());
    }

    anchor_info.bar_layer_surface.get_popup(&xdg_popup);
    info.positioner.destroy();
    surface.commit();

    match &kind {
        FloatingKind::Popup(target) => {
            state.surface_to_id.insert(
                surface.clone(),
                (target.module_id(), anchor_info.target_monitor_id.clone(), SurfaceKind::Popup),
            );
        }
        FloatingKind::Panel(target) => {
            state.surface_to_id.insert(
                surface.clone(),
                (target.module_id(), anchor_info.target_monitor_id.clone(), SurfaceKind::Panel),
            );
        }
        FloatingKind::Tooltip => {}
    }

    state.floating_surfaces.insert(
        kind,
        FloatingSurface {
            surface,
            xdg_surface,
            xdg_popup,
            shm_buffer: info.shm_buffer,
            size: info.size,
            layout,
            reposition_token: 0,
        },
    );

    Ok(())
}
