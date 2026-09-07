#![allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]

use super::floating_anchor::{handle_conflicts, resolve_anchor};
use super::floating_render::{calculate_floating_layout, create_positioner, render_to_buffer};
use super::state::WaylandState;
use super::types::FloatingSurface;
use crate::features::layout_engine::domain::{FloatingKind, StyledNode};
use crate::shared::events::core::SurfaceKind;
use crate::shared::primitives::geometry::{Scale, Size};
use crate::shared::primitives::MonitorId;
use crate::shared::wayland::adapters::shm::ShmBuffer;
use crate::shared::wayland::ports::DisplayServerError;
use wayland_client::QueueHandle;

pub(crate) fn show_floating(
    state: &mut WaylandState,
    qh: &QueueHandle<WaylandState>,
    kind: FloatingKind,
    monitor_id: Option<MonitorId>,
    anchor_rect: Option<crate::shared::primitives::geometry::Rect>,
    layout: StyledNode,
) -> Result<(), DisplayServerError> {
    if let Some(existing) = state.floating_surfaces.get(&kind)
        && existing.layout == layout
    {
        return Ok(());
    }

    if let FloatingKind::Popup(ref target) = kind {
        handle_conflicts(state, target);
    }

    let Some(anchor_info) = resolve_anchor(state, &kind, monitor_id, anchor_rect) else {
        return Ok(());
    };

    let font_family = crate::shared::config::domain::FontFamily::new("Inter".to_string());
    let font_size = crate::shared::config::domain::FontSize::new(12.0);
    let scale = Scale::new(anchor_info.bar_scale as f32);

    let (text_w, text_h, render_node) = calculate_floating_layout(state, &layout, scale, &font_family, font_size);
    let width = (((text_w as f32) * scale.value()).ceil() as u32).max(1);
    let height = (((text_h as f32) * scale.value()).ceil() as u32).max(1);
    let new_size = Size::new(width, height);

    if let Some(floating) = state.floating_surfaces.get_mut(&kind) {
        if floating.size == new_size {
            render_to_buffer(
                &mut floating.shm_buffer, &render_node, &mut state.font_system, &mut state.swash_cache,
                width, height, scale, font_family, font_size,
            );
            floating.surface.set_buffer_scale(anchor_info.bar_scale);
            floating.layout = layout;
            floating.surface.attach(Some(floating.shm_buffer.current_buffer()), 0, 0);
            floating.surface.damage_buffer(0, 0, width as i32, height as i32);
            floating.surface.commit();
            floating.shm_buffer.swap_buffers();
            return Ok(());
        }

        let shm = state.shm.as_ref().ok_or_else(|| DisplayServerError::ConnectionFailed {
            reason: "SHM not bound".to_string(),
        })?;
        let mut new_shm_buffer = ShmBuffer::new(shm, width, height, qh, state.app_env.xdg_runtime_dir().as_path())
            .map_err(|e| DisplayServerError::Internal(e.to_string()))?;
        render_to_buffer(
            &mut new_shm_buffer, &render_node, &mut state.font_system, &mut state.swash_cache,
            width, height, scale, font_family, font_size,
        );

        let xdg_wm_base = state.xdg_wm_base.as_ref().ok_or_else(|| DisplayServerError::ConnectionFailed {
            reason: "XDG WM Base not bound".to_string(),
        })?;
        let positioner = create_positioner(xdg_wm_base, qh, text_w, text_h, &anchor_info);
        floating.reposition_token = floating.reposition_token.wrapping_add(1);
        floating.xdg_popup.reposition(&positioner, floating.reposition_token);
        positioner.destroy();
        floating.shm_buffer = new_shm_buffer;
        floating.size = new_size;
        floating.layout = layout;
        floating.surface.set_buffer_scale(anchor_info.bar_scale);
        floating.surface.attach(Some(floating.shm_buffer.current_buffer()), 0, 0);
        floating.surface.damage_buffer(0, 0, width as i32, height as i32);
        floating.surface.commit();
        floating.shm_buffer.swap_buffers();
        return Ok(());
    }

    let shm = state.shm.as_ref().ok_or_else(|| DisplayServerError::ConnectionFailed {
        reason: "SHM not bound".to_string(),
    })?;
    let mut shm_buffer = ShmBuffer::new(shm, width, height, qh, state.app_env.xdg_runtime_dir().as_path())
        .map_err(|e| DisplayServerError::Internal(e.to_string()))?;
    render_to_buffer(
        &mut shm_buffer, &render_node, &mut state.font_system, &mut state.swash_cache,
        width, height, scale, font_family, font_size,
    );

    let xdg_wm_base = state.xdg_wm_base.as_ref().ok_or_else(|| DisplayServerError::ConnectionFailed {
        reason: "XDG WM Base not bound".to_string(),
    })?;
    let positioner = create_positioner(xdg_wm_base, qh, text_w, text_h, &anchor_info);

    let compositor = state.compositor.as_ref().ok_or_else(|| DisplayServerError::ConnectionFailed {
        reason: "Compositor not bound".to_string(),
    })?;
    let surface = compositor.create_surface(qh, ());
    surface.set_buffer_scale(anchor_info.bar_scale);
    let xdg_surface = xdg_wm_base.get_xdg_surface(&surface, qh, ());
    let xdg_popup = xdg_surface.get_popup(None, &positioner, qh, ());

    if let FloatingKind::Popup(_) = &kind
        && let (Some(seat), Some(serial)) = (state.seat.as_ref(), state.last_button_serial)
    {
        xdg_popup.grab(seat, serial.value());
    }

    anchor_info.bar_layer_surface.get_popup(&xdg_popup);
    positioner.destroy();
    surface.commit();

    match &kind {
        FloatingKind::Popup(target) => {
            state.surface_to_id.insert(
                surface.clone(),
                (target.module_id(), anchor_info.target_monitor_id, SurfaceKind::Popup),
            );
        }
        FloatingKind::Panel(target) => {
            state.surface_to_id.insert(
                surface.clone(),
                (target.module_id(), anchor_info.target_monitor_id, SurfaceKind::Panel),
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
            shm_buffer,
            size: new_size,
            layout,
            reposition_token: 0,
        },
    );

    Ok(())
}

pub(crate) fn hide_floating(
    state: &mut WaylandState,
    kind: &FloatingKind,
) -> Result<(), DisplayServerError> {
    if let Some(floating) = state.floating_surfaces.remove(kind) {
        state.surface_to_id.remove(&floating.surface);
    }
    Ok(())
}
