use super::floating_anchor::{handle_conflicts, resolve_anchor};
use super::floating_setup::{create_new_floating, update_existing_floating, FloatingPlacement};
use super::state::WaylandState;
use crate::features::layout_engine::domain::FloatingKind;
use crate::shared::primitives::geometry::{Rect, Size};
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::primitives::{MonitorId, PopupOffset};
use crate::shared::wayland::ports::DisplayServerError;
use wayland_client::QueueHandle;

/// An already laid-out and painted floating surface, as the module produced
/// it — nothing left for Wayland to compute, only to place and blit.
pub(crate) struct FloatingPayload<'a> {
    pub(crate) buffer: &'a RenderBuffer,
    pub(crate) logical_size: Size,
    pub(crate) offset: Option<PopupOffset>,
}

/// Shows or updates a floating surface. This is placement and blitting
/// only — layout and painting happen once, in the module that owns the
/// content, before this is ever called.
pub(crate) fn show_floating(
    state: &mut WaylandState,
    qh: &QueueHandle<WaylandState>,
    kind: FloatingKind,
    monitor_id: Option<MonitorId>,
    anchor_rect: Option<Rect>,
    payload: &FloatingPayload<'_>,
) -> Result<(), DisplayServerError> {
    if let FloatingKind::Popup(ref target) = kind {
        handle_conflicts(state, target);
    }

    let Some(anchor_info) = resolve_anchor(state, &kind, monitor_id, anchor_rect) else {
        return Ok(());
    };

    let effective_offset = match kind {
        FloatingKind::Popup(_) => payload.offset.unwrap_or_else(|| {
            state.hub.config_rx().borrow().popup().offset()
        }),
        _ => payload.offset.unwrap_or_default(),
    };

    let placement = FloatingPlacement {
        size: *payload.buffer.size(),
        text_w: i32::try_from(payload.logical_size.width()).unwrap_or(1),
        text_h: i32::try_from(payload.logical_size.height()).unwrap_or(1),
        offset: effective_offset,
    };

    if state.floating_surfaces.contains_key(&kind) {
        update_existing_floating(state, qh, &kind, payload.buffer, &anchor_info, &placement)
    } else {
        create_new_floating(state, qh, kind, payload.buffer, &anchor_info, &placement)
    }
}

pub(crate) fn hide_floating(state: &mut WaylandState, kind: &FloatingKind) {
    if let Some(floating) = state.floating_surfaces.remove(kind) {
        state.surface_to_id.remove(&floating.surface);
    }
}
