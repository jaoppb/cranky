use crate::features::layout_engine::domain::RenderNode;
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::geometry::{Position, Rect, Scale, Size};
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::rendering::ports::canvas::CanvasFactory;

#[allow(
    clippy::as_conversions,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn paint_pipeline<F: CanvasFactory>(
    render_node: &RenderNode,
    current_bounds: Option<Rect>,
    scale: Scale,
    canvas_factory: &mut F,
) -> Option<(RenderBuffer, Position)> {
    let bounds = current_bounds.filter(|b| b.width() > 0 && b.height() > 0)?;

    let default_font_family = FontFamily::new(String::new());
    let default_font_size = FontSize::new(14.0);

    let phys_w = ((bounds.width() as f32) * scale.value()).ceil().max(1.0) as u32;
    let phys_h = ((bounds.height() as f32) * scale.value()).ceil().max(1.0) as u32;
    let phys_size = Size::new(phys_w, phys_h);

    let width = usize::try_from(phys_w).unwrap_or(0);
    let height = usize::try_from(phys_h).unwrap_or(0);
    let len = width.saturating_mul(height).saturating_mul(4);
    let mut data = vec![0u8; len];
    {
        let mut canvas = canvas_factory.create_canvas(
            &mut data,
            phys_size,
            scale,
            default_font_family,
            default_font_size,
        );
        render_node.render_to_canvas(&mut canvas);
    }

    let render_buf = RenderBuffer::new(data, phys_size);
    let position = Position::new(bounds.x(), bounds.y());
    Some((render_buf, position))
}
