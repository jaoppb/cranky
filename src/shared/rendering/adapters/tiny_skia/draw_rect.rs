use super::canvas::TinySkiaCosmicCanvas;
use super::paint::get_paint;
use crate::shared::primitives::color::DrawingColor;
use crate::shared::primitives::geometry::{LogicalPx, Position, Size};
use tiny_skia::{FillRule, LineCap, LineJoin, PathBuilder, Rect, Stroke, Transform};

impl TinySkiaCosmicCanvas<'_> {
    #[allow(clippy::many_single_char_names)]
    pub(crate) fn draw_rect_impl(
        &mut self,
        x: LogicalPx,
        y: LogicalPx,
        width: LogicalPx,
        height: LogicalPx,
        color: &DrawingColor,
        radius: LogicalPx,
    ) {
        let Some(pixmap) = &mut self.pixmap else {
            return;
        };
        let physical_x = x.apply_scale(&self.scale).value();
        let physical_y = y.apply_scale(&self.scale).value();
        let physical_w = width.apply_scale(&self.scale).value();
        let physical_h = height.apply_scale(&self.scale).value();

        if let Some(physical_rect) = Rect::from_xywh(physical_x, physical_y, physical_w, physical_h)
        {
            let paint = get_paint(color, physical_rect);
            let r = radius
                .apply_scale(&self.scale)
                .value()
                .min(physical_rect.width() / 2.0)
                .min(physical_rect.height() / 2.0);

            if r <= 0.0 {
                pixmap.fill_rect(physical_rect, &paint, Transform::identity(), None);
            } else {
                let mut pb = PathBuilder::new();
                let (rx, ry, rw, rh) = (
                    physical_rect.left(),
                    physical_rect.top(),
                    physical_rect.width(),
                    physical_rect.height(),
                );
                pb.move_to(rx + r, ry);
                pb.line_to(rx + rw - r, ry);
                pb.quad_to(rx + rw, ry, rx + rw, ry + r);
                pb.line_to(rx + rw, ry + rh - r);
                pb.quad_to(rx + rw, ry + rh, rx + rw - r, ry + rh);
                pb.line_to(rx + r, ry + rh);
                pb.quad_to(rx, ry + rh, rx, ry + rh - r);
                pb.line_to(rx, ry + r);
                pb.quad_to(rx, ry, rx + r, ry);
                pb.close();

                if let Some(path) = pb.finish() {
                    pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
                }
            }
        }
    }

    #[allow(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        clippy::many_single_char_names
    )]
    pub(crate) fn draw_border_impl(
        &mut self,
        position: Position,
        size: Size,
        color: &DrawingColor,
        radius: LogicalPx,
        border_size: LogicalPx,
    ) {
        let Some(pixmap) = &mut self.pixmap else {
            return;
        };
        let x = LogicalPx::new(position.x() as f32);
        let y = LogicalPx::new(position.y() as f32);
        let width = LogicalPx::new(size.width() as f32);
        let height = LogicalPx::new(size.height() as f32);
        let physical_x = x.apply_scale(&self.scale).value();
        let physical_y = y.apply_scale(&self.scale).value();
        let physical_w = width.apply_scale(&self.scale).value();
        let physical_h = height.apply_scale(&self.scale).value();
        let stroke_w = border_size.apply_scale(&self.scale).value();

        if stroke_w <= 0.0 {
            return;
        }

        if let Some(physical_rect) = Rect::from_xywh(physical_x, physical_y, physical_w, physical_h)
        {
            let paint = get_paint(color, physical_rect);
            let stroke = Stroke {
                width: stroke_w,
                miter_limit: 4.0,
                line_cap: LineCap::Butt,
                line_join: LineJoin::Miter,
                dash: None,
            };

            let half_stroke = stroke_w / 2.0;
            let (bx, by, bw, bh) = (
                physical_rect.left() + half_stroke,
                physical_rect.top() + half_stroke,
                (physical_rect.width() - stroke_w).max(0.0),
                (physical_rect.height() - stroke_w).max(0.0),
            );

            let max_r = (bw / 2.0).min(bh / 2.0);
            let r = (radius.apply_scale(&self.scale).value() - half_stroke).clamp(0.0, max_r);

            let mut pb = PathBuilder::new();
            if r <= 0.0 {
                pb.move_to(bx, by);
                pb.line_to(bx + bw, by);
                pb.line_to(bx + bw, by + bh);
                pb.line_to(bx, by + bh);
            } else {
                pb.move_to(bx + r, by);
                pb.line_to(bx + bw - r, by);
                pb.quad_to(bx + bw, by, bx + bw, by + r);
                pb.line_to(bx + bw, by + bh - r);
                pb.quad_to(bx + bw, by + bh, bx + bw - r, by + bh);
                pb.line_to(bx + r, by + bh);
                pb.quad_to(bx, by + bh, bx, by + bh - r);
                pb.line_to(bx, by + r);
                pb.quad_to(bx, by, bx + r, by);
            }
            pb.close();

            if let Some(path) = pb.finish() {
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
        }
    }
}
