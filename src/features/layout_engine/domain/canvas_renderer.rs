use super::render_node::RenderNode;
use crate::features::styling::domain::{ComputedStyle, Orientation};
use crate::shared::primitives::color::DrawingColor;
use crate::shared::primitives::geometry::{LogicalPx, Position, Rect, Size};
use crate::shared::rendering::ports::canvas::Canvas;

#[allow(
    clippy::as_conversions,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation
)]
fn draw_bg_and_border(
    canvas: &mut dyn Canvas,
    rect: &Rect,
    style: &ComputedStyle,
    bg_override: Option<&DrawingColor>,
) {
    let bg = bg_override.or_else(|| style.background());
    if let Some(c) = bg {
        canvas.draw_rect(
            LogicalPx::new(rect.x() as f32),
            LogicalPx::new(rect.y() as f32),
            LogicalPx::new(rect.width() as f32),
            LogicalPx::new(rect.height() as f32),
            c.clone(),
            LogicalPx::new(style.border_radius().map_or(0.0, |r| r.value())),
        );
    }
    if let (Some(size), Some(color)) = (style.border_size(), style.border_color()) {
        canvas.draw_border(
            Position::new(rect.x(), rect.y()),
            Size::new(rect.width(), rect.height()),
            color.clone(),
            LogicalPx::new(style.border_radius().map_or(0.0, |r| r.value())),
            LogicalPx::new(size.value()),
        );
    }
}

impl RenderNode {
    #[allow(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::arithmetic_side_effects,
        clippy::too_many_lines
    )]
    pub fn render_to_canvas(&self, canvas: &mut dyn Canvas) {
        match self {
            Self::Flex {
                rect,
                children,
                style,
                ..
            }
            | Self::Grid {
                rect,
                children,
                style,
                ..
            } => {
                draw_bg_and_border(canvas, rect, style, None);
                for child in children {
                    child.render_to_canvas(canvas);
                }
            }
            Self::Progress {
                rect,
                value,
                orientation,
                style,
                ..
            } => {
                draw_bg_and_border(canvas, rect, style, None);
                let fill_color = style.accent_color().or_else(|| style.color());
                if let Some(fill) = fill_color {
                    let r = LogicalPx::new(style.border_radius().map_or(0.0, |rad| rad.value()));
                    let clamped = value.value().clamp(0.0, 1.0);
                    match orientation {
                        Orientation::Horizontal => {
                            let fill_w = (rect.width() as f32 * clamped).round();
                            if fill_w > 0.0 {
                                canvas.draw_rect(
                                    LogicalPx::new(rect.x() as f32),
                                    LogicalPx::new(rect.y() as f32),
                                    LogicalPx::new(fill_w),
                                    LogicalPx::new(rect.height() as f32),
                                    fill.clone(),
                                    r,
                                );
                            }
                        }
                        Orientation::Vertical => {
                            let fill_h = (rect.height() as f32 * clamped).round();
                            let fill_y = rect.y() as f32 + (rect.height() as f32 - fill_h);
                            if fill_h > 0.0 {
                                canvas.draw_rect(
                                    LogicalPx::new(rect.x() as f32),
                                    LogicalPx::new(fill_y),
                                    LogicalPx::new(rect.width() as f32),
                                    LogicalPx::new(fill_h),
                                    fill.clone(),
                                    r,
                                );
                            }
                        }
                    }
                }
            }
            Self::Rect { rect, style, .. } => {
                draw_bg_and_border(
                    canvas,
                    rect,
                    style,
                    style.background().or_else(|| style.color()),
                );
            }
            Self::Module { rect, style, .. } => {
                draw_bg_and_border(canvas, rect, style, None);
            }
            Self::Text {
                rect, text, style, ..
            } => {
                draw_bg_and_border(canvas, rect, style, None);
                let text_color = style.color().cloned().unwrap_or_else(|| {
                    DrawingColor::Solid(crate::shared::primitives::color::Color::new(
                        255, 255, 255, 255,
                    ))
                });
                canvas.draw_text(
                    text.as_str(),
                    style.font_family(),
                    style.font_size(),
                    text_color,
                    Position::new(rect.x(), rect.y()),
                );
            }
            Self::Image {
                rect,
                data,
                pixel_size,
                ..
            } => {
                let logical_size = Size::new(rect.width(), rect.height());
                canvas.draw_image(data, *pixel_size, logical_size, Position::new(rect.x(), rect.y()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::render_node::RenderNode;
    use crate::features::styling::domain::{ComputedStyle, Orientation, ProgressValue};
    use crate::features::vdom::domain::{NodePath, TextContent};
    use crate::shared::primitives::color::{Color, DrawingColor};
    use crate::shared::primitives::geometry::{Position, Rect, Size};
    use crate::shared::primitives::BinaryData;
    use crate::shared::rendering::ports::canvas::MockCanvas;

    #[test]
    fn test_render_to_canvas() {
        let mut canvas = MockCanvas::new();
        canvas.expect_draw_rect().times(1).return_const(());

        let mut rect_style = ComputedStyle::default();
        rect_style.set_background(DrawingColor::Solid(Color::new(255, 255, 255, 255)));
        let rect = RenderNode::Rect {
            path: NodePath::root(),
            rect: Rect::new(Position::new(0, 0), Size::new(10, 10)),
            style: rect_style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
        };

        rect.render_to_canvas(&mut canvas);

        let mut canvas = MockCanvas::new();
        canvas.expect_draw_rect().times(1).return_const(());
        let mut flex_style = ComputedStyle::default();
        flex_style.set_background(DrawingColor::Solid(Color::new(0, 0, 0, 255)));
        let flex = RenderNode::Flex {
            path: NodePath::root(),
            rect: Rect::new(Position::new(0, 0), Size::new(10, 10)),
            children: vec![],
            style: flex_style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
        };
        flex.render_to_canvas(&mut canvas);

        let mut canvas = MockCanvas::new();
        canvas.expect_draw_text().times(1).return_const(());
        let mut text_style = ComputedStyle::default();
        text_style.set_color(DrawingColor::Solid(Color::new(0, 0, 0, 255)));
        let text = RenderNode::Text {
            path: NodePath::root(),
            rect: Rect::new(Position::new(0, 0), Size::new(10, 10)),
            text: TextContent::new("test".to_string()),
            style: text_style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
        };
        text.render_to_canvas(&mut canvas);

        let mut canvas = MockCanvas::new();
        canvas.expect_draw_rect().times(2).return_const(());
        let mut prog_style = ComputedStyle::default();
        prog_style.set_background(DrawingColor::Solid(Color::new(0, 0, 0, 255)));
        prog_style.set_accent_color(DrawingColor::Solid(Color::new(255, 0, 0, 255)));
        let progress = RenderNode::Progress {
            path: NodePath::root(),
            rect: Rect::new(Position::new(0, 0), Size::new(100, 10)),
            value: ProgressValue::new(0.5).unwrap(),
            orientation: Orientation::Horizontal,
            style: prog_style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
        };
        progress.render_to_canvas(&mut canvas);

        let mut canvas = MockCanvas::new();
        canvas.expect_draw_image().times(1).return_const(());
        let image = RenderNode::Image {
            path: NodePath::root(),
            rect: Rect::new(Position::new(0, 0), Size::new(10, 10)),
            data: BinaryData::new(vec![0, 0, 0, 0]),
            pixel_size: Size::new(1, 1),
            tooltip: None,
            popup: None,
        };
        image.render_to_canvas(&mut canvas);
    }
}
