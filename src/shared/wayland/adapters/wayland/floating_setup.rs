use super::floating_anchor::{AnchorInfo, FloatingParentRole};
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

pub(super) use super::floating_update::update_existing_floating;

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

pub(super) fn create_new_floating(
    state: &mut WaylandState,
    qh: &QueueHandle<WaylandState>,
    kind: FloatingKind,
    buffer: &RenderBuffer,
    anchor_info: &AnchorInfo,
    placement: &FloatingPlacement,
    parent: Option<FloatingKind>,
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
        parent,
    )
}

fn setup_new_floating(
    state: &mut WaylandState,
    qh: &QueueHandle<WaylandState>,
    kind: FloatingKind,
    anchor_info: &AnchorInfo,
    info: NewFloatingInfo,
    parent: Option<FloatingKind>,
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
    // A popup nested inside another floating surface (decision 7) names its
    // parent directly; a top-level one leaves it unset and relies on
    // `zwlr_layer_surface_v1.get_popup` below instead.
    let parent_xdg_surface = match &anchor_info.parent_role {
        FloatingParentRole::Popup(p) => Some(p),
        FloatingParentRole::Layer(_) => None,
    };
    let xdg_popup = xdg_surface.get_popup(parent_xdg_surface, &info.positioner, qh, ());

    if let FloatingKind::Popup(_) = &kind
        && let (Some(seat), Some(serial)) = (state.seat.as_ref(), state.last_button_serial)
    {
        xdg_popup.grab(seat, serial.value());
    }

    if let FloatingParentRole::Layer(layer) = &anchor_info.parent_role {
        layer.get_popup(&xdg_popup);
    }
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
            module_surfaces: std::collections::HashMap::new(),
            parent,
        },
    );

    Ok(())
}
