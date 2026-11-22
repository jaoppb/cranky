//! Last pipeline phase: paints a laid-out tree into a buffer of physical pixels (4 bytes each) at
//! the monitor's scale.
use crate::features::layout_engine::domain::RenderNode;
use crate::shared::primitives::geometry::{Position, Rect, Scale, Size};
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::rendering::ports::canvas::CanvasFactory;
use crate::shared::rendering::ports::draw::PaintTarget;
use crate::shared::rendering::ports::font::Font;
use crate::utils::{f32_to_u32, f64_to_f32};

/// A canvas factory together with the output scale of the monitor its canvases paint for.
#[derive(Debug)]
pub(crate) struct ScaledCanvasFactory<'a, F: CanvasFactory> {
    factory: &'a mut F,
    scale: Scale,
}

impl<'a, F: CanvasFactory> ScaledCanvasFactory<'a, F> {
    #[must_use]
    pub(crate) const fn new(factory: &'a mut F, scale: Scale) -> Self {
        Self { factory, scale }
    }
}

pub(crate) fn paint_pipeline<F: CanvasFactory>(
    render_node: &RenderNode,
    current_bounds: Option<Rect>,
    canvases: ScaledCanvasFactory<'_, F>,
) -> Option<(RenderBuffer, Position)> {
    let ScaledCanvasFactory {
        factory: canvas_factory,
        scale,
    } = canvases;
    let bounds = current_bounds.filter(|rect| rect.width() > 0 && rect.height() > 0)?;

    let w_f32 = f64_to_f32(f64::from(bounds.width()));
    let h_f32 = f64_to_f32(f64::from(bounds.height()));
    let phys_w = f32_to_u32((w_f32 * scale.value()).ceil().max(1.0));
    let phys_h = f32_to_u32((h_f32 * scale.value()).ceil().max(1.0));
    let phys_size = Size::new(phys_w, phys_h);

    let width = usize::try_from(phys_w).unwrap_or(0);
    let height = usize::try_from(phys_h).unwrap_or(0);
    let len = width.saturating_mul(height).saturating_mul(4);
    let mut data = vec![0u8; len];
    {
        let mut canvas = canvas_factory.create_canvas(
            PaintTarget::new(&mut data, phys_size, scale),
            Font::fallback(),
        );
        render_node.render_to_canvas(&mut canvas);
    }

    let render_buf = RenderBuffer::new(data, phys_size);
    let position = Position::new(bounds.x(), bounds.y());
    Some((render_buf, position))
}
