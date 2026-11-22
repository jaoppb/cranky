use super::canvas::TinySkiaCosmicCanvas;
use super::paint::get_paint;
use super::physical_box::PhysicalBox;
use crate::shared::primitives::color::DrawingColor;
use crate::shared::primitives::geometry::Scale;
use crate::shared::rendering::ports::draw::{BorderStroke, RoundedRect};
use tiny_skia::{FillRule, LineCap, LineJoin, Rect, Stroke, Transform};

impl TinySkiaCosmicCanvas<'_> {
    pub(crate) fn draw_rect_impl(&mut self, shape: RoundedRect, color: &DrawingColor) {
        let Some(pixmap) = &mut self.pixmap else {
            return;
        };
        let Some(physical_rect) = scaled_rect(&shape, self.scale) else {
            return;
        };
        let paint = get_paint(color, physical_rect);
        let clamped_radius = shape
            .radius()
            .apply_scale(self.scale)
            .value()
            .min(physical_rect.width() / 2.0)
            .min(physical_rect.height() / 2.0);

        if clamped_radius <= 0.0 {
            pixmap.fill_rect(physical_rect, &paint, Transform::identity(), None);
        } else if let Some(path) = PhysicalBox::covering(physical_rect).rounded_path(clamped_radius)
        {
            pixmap.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    }

    pub(crate) fn draw_border_impl(&mut self, shape: RoundedRect, border: &BorderStroke) {
        let Some(pixmap) = &mut self.pixmap else {
            return;
        };
        let stroke_w = border.width().apply_scale(self.scale).value();
        if stroke_w <= 0.0 {
            return;
        }
        let Some(physical_rect) = scaled_rect(&shape, self.scale) else {
            return;
        };
        let paint = get_paint(border.color(), physical_rect);

        let half_stroke = stroke_w / 2.0;
        let bounds = PhysicalBox::inside_stroke(physical_rect, stroke_w);
        let clamped_radius = (shape.radius().apply_scale(self.scale).value() - half_stroke)
            .clamp(0.0, bounds.max_radius());

        let path = if clamped_radius <= 0.0 {
            bounds.square_path()
        } else {
            bounds.rounded_path(clamped_radius)
        };
        if let Some(path) = path {
            let stroke = border_stroke(stroke_w);
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }
}

/// The rectangle of `shape` in physical pixels at `scale`, or `None` when it is not a valid one.
fn scaled_rect(shape: &RoundedRect, scale: Scale) -> Option<Rect> {
    let physical_x = shape.x().apply_scale(scale).value();
    let physical_y = shape.y().apply_scale(scale).value();
    let physical_w = shape.width().apply_scale(scale).value();
    let physical_h = shape.height().apply_scale(scale).value();
    Rect::from_xywh(physical_x, physical_y, physical_w, physical_h)
}

/// A solid stroke `width` pixels wide with butt caps and mitred joins.
const fn border_stroke(width: f32) -> Stroke {
    Stroke {
        width,
        miter_limit: 4.0,
        line_cap: LineCap::Butt,
        line_join: LineJoin::Miter,
        dash: None,
    }
}
