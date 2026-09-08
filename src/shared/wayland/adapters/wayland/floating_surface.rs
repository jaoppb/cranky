use super::floating_anchor::{handle_conflicts, resolve_anchor};
use super::floating_render::calculate_floating_layout;
use super::floating_setup::{create_new_floating, update_existing_floating};
use super::state::WaylandState;
use crate::features::layout_engine::domain::{FloatingKind, RenderNode, StyledNode};
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::geometry::{Scale, Size};
use crate::shared::primitives::{MonitorId, PopupOffset};
use crate::shared::wayland::ports::DisplayServerError;
use wayland_client::QueueHandle;

pub(super) struct FloatingRenderContext<'a> {
    pub(super) render_node: &'a RenderNode,
    pub(super) scale: Scale,
    pub(super) text_w: i32,
    pub(super) text_h: i32,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) size: Size,
    pub(super) font_family: FontFamily,
    pub(super) font_size: FontSize,
    pub(super) effective_offset: PopupOffset,
}

pub(crate) fn show_floating(
    state: &mut WaylandState,
    qh: &QueueHandle<WaylandState>,
    kind: FloatingKind,
    monitor_id: Option<MonitorId>,
    anchor_rect: Option<crate::shared::primitives::geometry::Rect>,
    layout: StyledNode,
    offset: Option<PopupOffset>,
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

    let effective_offset = match kind {
        FloatingKind::Popup(_) => offset.unwrap_or_else(|| {
            state.hub.config_rx().borrow().popup().offset()
        }),
        _ => offset.unwrap_or_default(),
    };

    let font_family = FontFamily::new("Inter".to_string());
    let font_size = FontSize::new(12.0);
    let bar_scale_f32 = i16::try_from(anchor_info.bar_scale).map_or(1.0, f32::from);
    let scale = Scale::new(bar_scale_f32);

    let (text_w, text_h, render_node) =
        calculate_floating_layout(state, &layout, scale, &font_family, font_size);
    let layout_width = i16::try_from(text_w.clamp(0, 32767)).map_or(1.0, f32::from);
    let layout_height = i16::try_from(text_h.clamp(0, 32767)).map_or(1.0, f32::from);
    let width = crate::utils::f32_to_u32((layout_width * scale.value()).ceil()).max(1);
    let height = crate::utils::f32_to_u32((layout_height * scale.value()).ceil()).max(1);
    let ctx = FloatingRenderContext {
        render_node: &render_node,
        scale,
        text_w,
        text_h,
        width,
        height,
        size: Size::new(width, height),
        font_family,
        font_size,
        effective_offset,
    };

    if state.floating_surfaces.contains_key(&kind) {
        update_existing_floating(state, qh, &kind, layout, &anchor_info, &ctx)
    } else {
        create_new_floating(state, qh, kind, layout, &anchor_info, &ctx)
    }
}

pub(crate) fn hide_floating(state: &mut WaylandState, kind: &FloatingKind) {
    if let Some(floating) = state.floating_surfaces.remove(kind) {
        state.surface_to_id.remove(&floating.surface);
    }
}
