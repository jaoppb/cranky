use super::canvas::TinySkiaCosmicCanvas;
use super::paint::get_paint;
use crate::shared::primitives::color::DrawingColor;
use crate::shared::primitives::geometry::{LogicalPx, Position, Size};
use tiny_skia::{FillRule, LineCap, LineJoin, PathBuilder, Rect, Stroke, Transform};

impl TinySkiaCosmicCanvas<'_> {
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
            let clamped_radius = radius
                .apply_scale(&self.scale)
                .value()
                .min(physical_rect.width() / 2.0)
                .min(physical_rect.height() / 2.0);

            if clamped_radius <= 0.0 {
                pixmap.fill_rect(physical_rect, &paint, Transform::identity(), None);
            } else {
                let mut builder = PathBuilder::new();
                let (rect_x, rect_y, rect_w, rect_h) = (
                    physical_rect.left(),
                    physical_rect.top(),
                    physical_rect.width(),
                    physical_rect.height(),
                );
                builder.move_to(rect_x + clamped_radius, rect_y);
                builder.line_to(rect_x + rect_w - clamped_radius, rect_y);
                builder.quad_to(rect_x + rect_w, rect_y, rect_x + rect_w, rect_y + clamped_radius);
                builder.line_to(rect_x + rect_w, rect_y + rect_h - clamped_radius);
                builder.quad_to(rect_x + rect_w, rect_y + rect_h, rect_x + rect_w - clamped_radius, rect_y + rect_h);
                builder.line_to(rect_x + clamped_radius, rect_y + rect_h);
                builder.quad_to(rect_x, rect_y + rect_h, rect_x, rect_y + rect_h - clamped_radius);
                builder.line_to(rect_x, rect_y + clamped_radius);
                builder.quad_to(rect_x, rect_y, rect_x + clamped_radius, rect_y);
                builder.close();

                if let Some(path) = builder.finish() {
                    pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
                }
            }
        }
    }

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
        let pos_x = f32::from(i16::try_from(position.x()).unwrap_or(0));
        let pos_y = f32::from(i16::try_from(position.y()).unwrap_or(0));
        let width_val = f32::from(u16::try_from(size.width()).unwrap_or(0));
        let height_val = f32::from(u16::try_from(size.height()).unwrap_or(0));
        let x = LogicalPx::new(pos_x);
        let y = LogicalPx::new(pos_y);
        let width = LogicalPx::new(width_val);
        let height = LogicalPx::new(height_val);
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
            let (border_x, border_y, border_w, border_h) = (
                physical_rect.left() + half_stroke,
                physical_rect.top() + half_stroke,
                (physical_rect.width() - stroke_w).max(0.0),
                (physical_rect.height() - stroke_w).max(0.0),
            );

            let max_radius = (border_w / 2.0).min(border_h / 2.0);
            let clamped_radius = (radius.apply_scale(&self.scale).value() - half_stroke).clamp(0.0, max_radius);

            let mut builder = PathBuilder::new();
            if clamped_radius <= 0.0 {
                builder.move_to(border_x, border_y);
                builder.line_to(border_x + border_w, border_y);
                builder.line_to(border_x + border_w, border_y + border_h);
                builder.line_to(border_x, border_y + border_h);
            } else {
                builder.move_to(border_x + clamped_radius, border_y);
                builder.line_to(border_x + border_w - clamped_radius, border_y);
                builder.quad_to(border_x + border_w, border_y, border_x + border_w, border_y + clamped_radius);
                builder.line_to(border_x + border_w, border_y + border_h - clamped_radius);
                builder.quad_to(border_x + border_w, border_y + border_h, border_x + border_w - clamped_radius, border_y + border_h);
                builder.line_to(border_x + clamped_radius, border_y + border_h);
                builder.quad_to(border_x, border_y + border_h, border_x, border_y + border_h - clamped_radius);
                builder.line_to(border_x, border_y + clamped_radius);
                builder.quad_to(border_x, border_y, border_x + clamped_radius, border_y);
            }
            builder.close();

            if let Some(path) = builder.finish() {
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
        }
    }
}
