mod canvas;
mod fonts;
mod placement;
mod text;

use crate::shared::primitives::geometry::{Scale, Size};
use crate::shared::rendering::ports::draw::PaintTarget;

/// A target over all of `pixmap`, drawn into at `scale`.
fn target(pixmap: &mut tiny_skia::Pixmap, scale: f32) -> PaintTarget<'_> {
    let size = Size::new(pixmap.width(), pixmap.height());
    PaintTarget::new(pixmap.data_mut(), size, Scale::new(scale))
}
