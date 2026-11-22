//! A box in physical pixels and the outlines the canvas fills or strokes along it.
use tiny_skia::{Path, PathBuilder, Rect};

/// A box in physical pixels, kept as its origin and size so the outline is traced through
/// exactly the sums of those four values.
#[derive(Clone, Copy)]
pub(super) struct PhysicalBox {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl PhysicalBox {
    /// The box `rect` covers.
    pub(super) fn covering(rect: Rect) -> Self {
        Self {
            x: rect.left(),
            y: rect.top(),
            width: rect.width(),
            height: rect.height(),
        }
    }

    /// The box the centre line of a stroke `stroke_width` wide runs along to stay inside `rect`.
    pub(super) fn inside_stroke(rect: Rect, stroke_width: f32) -> Self {
        let half_stroke = stroke_width / 2.0;
        Self {
            x: rect.left() + half_stroke,
            y: rect.top() + half_stroke,
            width: (rect.width() - stroke_width).max(0.0),
            height: (rect.height() - stroke_width).max(0.0),
        }
    }

    /// The largest corner radius the box holds.
    pub(super) fn max_radius(self) -> f32 {
        (self.width / 2.0).min(self.height / 2.0)
    }

    /// The closed outline of the box with square corners.
    pub(super) fn square_path(self) -> Option<Path> {
        let Self {
            x,
            y,
            width,
            height,
        } = self;
        let mut builder = PathBuilder::new();
        builder.move_to(x, y);
        builder.line_to(x + width, y);
        builder.line_to(x + width, y + height);
        builder.line_to(x, y + height);
        builder.close();
        builder.finish()
    }

    /// The closed outline of the box with corners rounded by `radius`, as quadratic curves.
    pub(super) fn rounded_path(self, radius: f32) -> Option<Path> {
        let Self {
            x,
            y,
            width,
            height,
        } = self;
        let mut builder = PathBuilder::new();
        builder.move_to(x + radius, y);
        builder.line_to(x + width - radius, y);
        builder.quad_to(x + width, y, x + width, y + radius);
        builder.line_to(x + width, y + height - radius);
        builder.quad_to(x + width, y + height, x + width - radius, y + height);
        builder.line_to(x + radius, y + height);
        builder.quad_to(x, y + height, x, y + height - radius);
        builder.line_to(x, y + radius);
        builder.quad_to(x, y, x + radius, y);
        builder.close();
        builder.finish()
    }
}
