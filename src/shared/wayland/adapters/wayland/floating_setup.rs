use super::floating_anchor::AnchorInfo;
use super::floating_render::create_positioner;
use super::state::WaylandState;
use super::surface_handler::copy_surface_data;
use super::types::FloatingSurface;
use crate::features::layout_engine::domain::FloatingKind;
use crate::shared::events::core::SurfaceKind;
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::primitives::PopupOffset;
use crate::shared::wayland::adapters::shm::ShmBuffer;
use crate::shared::wayland::ports::DisplayServerError;
use wayland_client::QueueHandle;
use wayland_protocols::xdg::shell::client::xdg_positioner::XdgPositioner;

/// Everything Wayland needs to place a floating surface — no rendering
/// information, since the module already painted `buffer`. `size` is the
/// buffer's own physical (scaled) size; `text_w`/`text_h` are the logical
/// size the positioner works in.
pub(super) struct FloatingPlacement {
    pub(super) size: Size,
    pub(super) text_w: i32,
    pub(super) text_h: i32,
    pub(super) offset: PopupOffset,
}

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
    buffer: &RenderBuffer,
    anchor_info: &AnchorInfo,
    placement: &FloatingPlacement,
) -> Result<(), DisplayServerError> {
    let Some(floating) = state.floating_surfaces.get_mut(kind) else {
        return Ok(());
    };
    let w_i32 = i32::try_from(placement.size.width()).unwrap_or(0);
    let h_i32 = i32::try_from(placement.size.height()).unwrap_or(0);

    if floating.size == placement.size {
        copy_surface_data(&mut floating.shm_buffer, buffer.data());
        commit_surface(floating, anchor_info.bar_scale, w_i32, h_i32);
        return Ok(());
    }

    let shm = state.shm.as_ref().ok_or_else(|| DisplayServerError::ConnectionFailed {
        reason: "SHM not bound".to_string(),
    })?;
    let mut new_shm_buffer = ShmBuffer::new(
        shm,
        placement.size.width(),
        placement.size.height(),
        qh,
        state.app_env.xdg_runtime_dir().as_path(),
    )
    .map_err(|e| DisplayServerError::Internal(e.to_string()))?;
    copy_surface_data(&mut new_shm_buffer, buffer.data());

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
    floating.shm_buffer = new_shm_buffer;
    floating.size = placement.size;
    commit_surface(floating, anchor_info.bar_scale, w_i32, h_i32);
    Ok(())
}

pub(super) fn create_new_floating(
    state: &mut WaylandState,
    qh: &QueueHandle<WaylandState>,
    kind: FloatingKind,
    buffer: &RenderBuffer,
    anchor_info: &AnchorInfo,
    placement: &FloatingPlacement,
) -> Result<(), DisplayServerError> {
    let shm = state.shm.as_ref().ok_or_else(|| DisplayServerError::ConnectionFailed {
        reason: "SHM not bound".to_string(),
    })?;
    let mut shm_buffer = ShmBuffer::new(
        shm,
        placement.size.width(),
        placement.size.height(),
        qh,
        state.app_env.xdg_runtime_dir().as_path(),
    )
    .map_err(|e| DisplayServerError::Internal(e.to_string()))?;
    copy_surface_data(&mut shm_buffer, buffer.data());

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

    setup_new_floating(
        state,
        qh,
        kind,
        anchor_info,
        NewFloatingInfo {
            shm_buffer,
            size: placement.size,
            positioner,
        },
    )
}

fn setup_new_floating(
    state: &mut WaylandState,
    qh: &QueueHandle<WaylandState>,
    kind: FloatingKind,
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
            reposition_token: 0,
        },
    );

    Ok(())
}
