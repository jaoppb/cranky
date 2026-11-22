use super::floating_anchor::{handle_conflicts, resolve_anchor};
use super::floating_setup::{
    FloatingFrame, FloatingPlacement, create_new_floating, update_existing_floating,
};
use super::floating_teardown::{TeardownCause, teardown_floating};
use super::state::WaylandState;
use crate::features::layout_engine::domain::popup::ancestors;
use crate::features::layout_engine::domain::{FloatingKind, ShowFloating};
use crate::shared::wayland::ports::DisplayServerError;
use wayland_client::QueueHandle;

/// Shows or updates a floating surface. This is placement and blitting
/// only — layout and painting happen once, in the module that owns the
/// content, before this is ever called. `parent` is the floating surface
/// this one nests inside (decision 7), if any, fixed for `kind`'s whole
/// life on first show.
pub(super) fn show_floating(
    state: &mut WaylandState,
    queue_handle: &QueueHandle<WaylandState>,
    request: ShowFloating,
) -> Result<(), DisplayServerError> {
    let parent_of = |child_kind: &FloatingKind| -> Option<FloatingKind> {
        if *child_kind == request.kind {
            request.parent.clone()
        } else {
            state
                .floating_surfaces
                .get(child_kind)
                .and_then(|floating| floating.parent.clone())
        }
    };
    let Ok(ancestor_kinds) = ancestors(&request.kind, parent_of) else {
        tracing::warn!(kind = ?request.kind, "popup nesting cycle detected, dropping");
        return Ok(());
    };

    if let FloatingKind::Popup(ref target) = request.kind {
        handle_conflicts(state, target, &ancestor_kinds);
    }

    let Some(anchor) = resolve_anchor(state, &request) else {
        return Ok(());
    };

    let effective_offset = match request.kind {
        FloatingKind::Popup(_) => request
            .offset
            .unwrap_or_else(|| state.hub.config_rx().borrow().popup().offset()),
        _ => request.offset.unwrap_or_default(),
    };

    let placement = FloatingPlacement {
        anchor,
        text_w: i32::try_from(request.logical_size.width()).unwrap_or(1),
        text_h: i32::try_from(request.logical_size.height()).unwrap_or(1),
        offset: effective_offset,
    };
    let frame = FloatingFrame {
        kind: request.kind,
        parent: request.parent,
        buffer: request.buffer,
        placement,
    };

    if state.floating_surfaces.contains_key(&frame.kind) {
        update_existing_floating(state, queue_handle, frame)
    } else {
        create_new_floating(state, queue_handle, frame)
    }
}

pub(super) fn hide_floating(state: &mut WaylandState, kind: &FloatingKind) {
    // The module closed its own popup/panel — it already knows, so its own
    // owner gets no dismissal notice; any nested descendant still does
    // (decision 7), the same as every other teardown path.
    teardown_floating(state, kind, TeardownCause::OwnerClosed);
}
