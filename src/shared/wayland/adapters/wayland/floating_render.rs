#![allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]

use super::floating_anchor::AnchorInfo;
use super::state::WaylandState;
use crate::features::layout_engine::domain::StyledNode;
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::shared::primitives::geometry::{Rect, Scale, Size};
use crate::shared::rendering::adapters::tiny_skia::TinySkiaCosmicCanvas;
use crate::shared::wayland::adapters::shm::ShmBuffer;
use cosmic_text::{FontSystem, SwashCache};
use wayland_client::QueueHandle;
use wayland_protocols::xdg::shell::client::xdg_positioner::{
    Anchor as XdgAnchor, ConstraintAdjustment, Gravity as XdgGravity, XdgPositioner,
};
use wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase;

pub(crate) fn calculate_floating_layout(
    state: &mut WaylandState,
    layout: &StyledNode,
    scale: Scale,
    font_family: &crate::shared::config::domain::FontFamily,
    font_size: crate::shared::config::domain::FontSize,
) -> (i32, i32, crate::features::layout_engine::domain::RenderNode) {
    let mut measurer = crate::shared::rendering::adapters::tiny_skia::CosmicTextMeasurer::new(
        &mut state.font_system,
        scale,
        font_family.clone(),
        font_size,
    );
    let mut engine = crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter::new();
    if let Ok(render_node) = engine.calculate_layout(
        layout.clone(),
        &mut measurer,
        crate::shared::primitives::geometry::Position::new(0, 0),
    ) {
        let rect = render_node.rect();
        (rect.width() as i32, rect.height() as i32, render_node)
    } else {
        (
            1,
            1,
            crate::features::layout_engine::domain::RenderNode::Rect {
                path: crate::features::vdom::domain::NodePath::root(),
                rect: Rect::new(
                    crate::shared::primitives::geometry::Position::new(0, 0),
                    Size::new(1, 1),
                ),
                style: crate::features::styling::domain::ComputedStyle::default(),
                on_click: None,
                on_hover: None,
                tooltip: None,
                popup: None,
                panel: None,
            },
        )
    }
}

pub(crate) fn create_positioner(
    xdg_wm_base: &XdgWmBase,
    qh: &QueueHandle<WaylandState>,
    text_w: i32,
    text_h: i32,
    anchor_info: &AnchorInfo,
    offset: crate::shared::primitives::PopupOffset,
) -> XdgPositioner {
    let positioner = xdg_wm_base.create_positioner(qh, ());
    positioner.set_size(text_w.max(1), text_h.max(1));
    positioner.set_anchor_rect(
        anchor_info.anchor_x,
        anchor_info.anchor_y,
        anchor_info.anchor_w.max(1),
        anchor_info.anchor_h.max(1),
    );
    positioner.set_anchor(XdgAnchor::Bottom);
    positioner.set_gravity(XdgGravity::Bottom);
    positioner.set_offset(offset.dx(), offset.dy());
    positioner.set_constraint_adjustment(
        ConstraintAdjustment::SlideX | ConstraintAdjustment::SlideY | ConstraintAdjustment::FlipY,
    );
    positioner
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_to_buffer(
    shm_buffer: &mut ShmBuffer,
    render_node: &crate::features::layout_engine::domain::RenderNode,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    width: u32,
    height: u32,
    scale: Scale,
    font_family: crate::shared::config::domain::FontFamily,
    font_size: crate::shared::config::domain::FontSize,
) {
    let data = shm_buffer.mmap_mut();
    if let Some(pixmap) = tiny_skia::PixmapMut::from_bytes(data, width, height) {
        let mut actual_canvas = TinySkiaCosmicCanvas::new(
            pixmap,
            font_system,
            swash_cache,
            scale,
            font_family,
            font_size,
        );
        render_node.render_to_canvas(&mut actual_canvas);
    }
}
