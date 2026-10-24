use super::render_node::RenderNode;
use crate::features::styling::domain::{ComputedStyle, Orientation};
use crate::shared::primitives::color::DrawingColor;
use crate::shared::primitives::geometry::{LogicalPx, Position, Rect, Size};
use crate::shared::rendering::ports::canvas::Canvas;
use crate::utils::f64_to_i64;

fn draw_bg_and_border(
    canvas: &mut dyn Canvas,
    rect: &Rect,
    style: &ComputedStyle,
    bg_override: Option<&DrawingColor>,
) {
    let rx = f32::from(i16::try_from(rect.x()).unwrap_or(0));
    let ry = f32::from(i16::try_from(rect.y()).unwrap_or(0));
    let rw = f32::from(u16::try_from(rect.width()).unwrap_or(0));
    let rh = f32::from(u16::try_from(rect.height()).unwrap_or(0));
    let radius = LogicalPx::new(style.border_radius().map_or(0.0, |r| r.value()));

    let bg = bg_override.or_else(|| style.background());
    if let Some(c) = bg {
        canvas.draw_rect(
            LogicalPx::new(rx),
            LogicalPx::new(ry),
            LogicalPx::new(rw),
            LogicalPx::new(rh),
            c.clone(),
            radius,
        );
    }
    if let (Some(size), Some(color)) = (style.border_size(), style.border_color()) {
        canvas.draw_border(
            Position::new(rect.x(), rect.y()),
            Size::new(rect.width(), rect.height()),
            color.clone(),
            radius,
            LogicalPx::new(size.value()),
        );
    }
}

fn render_progress(
    canvas: &mut dyn Canvas,
    rect: &Rect,
    progress_val: f32,
    orientation: Orientation,
    style: &ComputedStyle,
) {
    draw_bg_and_border(canvas, rect, style, None);
    let fill_color = style.accent_color().or_else(|| style.color());
    let Some(fill) = fill_color else {
        return;
    };

    let r = LogicalPx::new(style.border_radius().map_or(0.0, |rad| rad.value()));
    let clamped = progress_val.clamp(0.0, 1.0);
    let rx = f32::from(i16::try_from(rect.x()).unwrap_or(0));
    let ry = f32::from(i16::try_from(rect.y()).unwrap_or(0));
    let rw = f32::from(u16::try_from(rect.width()).unwrap_or(0));
    let rh = f32::from(u16::try_from(rect.height()).unwrap_or(0));

    match orientation {
        Orientation::Horizontal => {
            let fill_w = (rw * clamped).round();
            if fill_w > 0.0 {
                canvas.draw_rect(
                    LogicalPx::new(rx),
                    LogicalPx::new(ry),
                    LogicalPx::new(fill_w),
                    LogicalPx::new(rh),
                    fill.clone(),
                    r,
                );
            }
        }
        Orientation::Vertical => {
            let fill_h = (rh * clamped).round();
            let fill_y = ry + (rh - fill_h);
            if fill_h > 0.0 {
                canvas.draw_rect(
                    LogicalPx::new(rx),
                    LogicalPx::new(fill_y),
                    LogicalPx::new(rw),
                    LogicalPx::new(fill_h),
                    fill.clone(),
                    r,
                );
            }
        }
    }
}

/// Where a text leaf's glyphs start: the box's origin moved in by its
/// padding, which layout already added to the box (see `leaf_size`).
fn text_origin(rect: &Rect, style: &ComputedStyle) -> Position {
    let inset = |v: f64| i32::try_from(f64_to_i64(v.max(0.0))).unwrap_or(0);
    let (left, top) = style
        .padding()
        .map_or((0, 0), |p| (inset(p.left()), inset(p.top())));
    Position::new(rect.x().saturating_add(left), rect.y().saturating_add(top))
}

fn render_text(
    canvas: &mut dyn Canvas,
    rect: &Rect,
    text: &crate::features::vdom::domain::TextContent,
    style: &ComputedStyle,
) {
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
        text_origin(rect, style),
    );
}

impl RenderNode {
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
                render_progress(canvas, rect, value.value(), *orientation, style);
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
                render_text(canvas, rect, text, style);
            }
            Self::Image {
                rect,
                data,
                pixel_size,
                ..
            } => {
                let logical_size = Size::new(rect.width(), rect.height());
                canvas.draw_image(
                    data,
                    *pixel_size,
                    logical_size,
                    Position::new(rect.x(), rect.y()),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::render_node::RenderNode;
    use super::super::style::BoxMargin;
    use crate::features::styling::domain::{ComputedStyle, Orientation, ProgressValue};
    use crate::features::vdom::domain::{NodePath, SurfaceSpace, TextContent};
    use crate::shared::primitives::BinaryData;
    use crate::shared::primitives::color::{Color, DrawingColor};
    use crate::shared::primitives::geometry::{Position, Rect, Size};
    use crate::shared::rendering::ports::canvas::MockCanvas;

    #[test]
    fn test_render_to_canvas() {
        let mut canvas = MockCanvas::new();
        canvas.expect_draw_rect().times(1).return_const(());

        let mut rect_style = ComputedStyle::default();
        rect_style.set_background(DrawingColor::Solid(Color::new(255, 255, 255, 255)));
        let rect = RenderNode::Rect {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            rect: Rect::new(Position::new(0, 0), Size::new(10, 10)),
            style: rect_style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        };

        rect.render_to_canvas(&mut canvas);

        let mut canvas = MockCanvas::new();
        canvas.expect_draw_rect().times(1).return_const(());
        let mut flex_style = ComputedStyle::default();
        flex_style.set_background(DrawingColor::Solid(Color::new(0, 0, 0, 255)));
        let flex = RenderNode::Flex {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            rect: Rect::new(Position::new(0, 0), Size::new(10, 10)),
            children: vec![],
            style: flex_style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        };
        flex.render_to_canvas(&mut canvas);

        let mut canvas = MockCanvas::new();
        canvas.expect_draw_text().times(1).return_const(());
        let mut text_style = ComputedStyle::default();
        text_style.set_color(DrawingColor::Solid(Color::new(0, 0, 0, 255)));
        let text = RenderNode::Text {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            rect: Rect::new(Position::new(0, 0), Size::new(10, 10)),
            text: TextContent::new("test".to_string()),
            style: text_style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        };
        text.render_to_canvas(&mut canvas);

        let mut canvas = MockCanvas::new();
        canvas.expect_draw_rect().times(2).return_const(());
        let mut prog_style = ComputedStyle::default();
        prog_style.set_background(DrawingColor::Solid(Color::new(0, 0, 0, 255)));
        prog_style.set_accent_color(DrawingColor::Solid(Color::new(255, 0, 0, 255)));
        let progress = RenderNode::Progress {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            rect: Rect::new(Position::new(0, 0), Size::new(100, 10)),
            value: ProgressValue::new(0.5).unwrap(),
            orientation: Orientation::Horizontal,
            style: prog_style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        };
        progress.render_to_canvas(&mut canvas);

        let mut canvas = MockCanvas::new();
        canvas.expect_draw_image().times(1).return_const(());
        let image = RenderNode::Image {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            rect: Rect::new(Position::new(0, 0), Size::new(10, 10)),
            data: BinaryData::new(vec![0, 0, 0, 0]),
            pixel_size: Size::new(1, 1),
            tooltip: None,
            popup: None,
            panel: None,
        };
        image.render_to_canvas(&mut canvas);
    }

    fn text_node(style: ComputedStyle) -> RenderNode {
        RenderNode::Text {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            rect: Rect::new(Position::new(20, 30), Size::new(30, 20)),
            text: TextContent::new("7".to_string()),
            style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        }
    }

    #[test]
    fn test_text_is_drawn_inside_its_padding() {
        let mut padded = ComputedStyle::default();
        padded.set_padding(BoxMargin::new(4.0, 4.0, 2.0, 2.0));
        let mut canvas = MockCanvas::new();
        canvas
            .expect_draw_text()
            .withf(|_, _, _, _, pos| *pos == Position::new(22, 34))
            .times(1)
            .return_const(());
        text_node(padded).render_to_canvas(&mut canvas);

        let mut canvas = MockCanvas::new();
        canvas
            .expect_draw_text()
            .withf(|_, _, _, _, pos| *pos == Position::new(20, 30))
            .times(1)
            .return_const(());
        text_node(ComputedStyle::default()).render_to_canvas(&mut canvas);
    }
}
