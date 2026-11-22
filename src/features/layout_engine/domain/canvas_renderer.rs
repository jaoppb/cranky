//! Paints a laid-out `RenderNode` tree onto a canvas: backgrounds, borders, text, progress bars
//! and images.
use super::render_node::RenderNode;
use super::text_placement::{text_alignment, text_origin};
use crate::features::styling::domain::{ComputedStyle, Orientation};
use crate::shared::config::domain::BorderRadius;
use crate::shared::primitives::color::DrawingColor;
use crate::shared::primitives::geometry::{LogicalPx, Position, Rect, Size};
use crate::shared::rendering::ports::canvas::Canvas;

fn draw_bg_and_border(
    canvas: &mut dyn Canvas,
    rect: &Rect,
    style: &ComputedStyle,
    bg_override: Option<&DrawingColor>,
) {
    let left = f32::from(i16::try_from(rect.x()).unwrap_or(0));
    let top = f32::from(i16::try_from(rect.y()).unwrap_or(0));
    let width = f32::from(u16::try_from(rect.width()).unwrap_or(0));
    let height = f32::from(u16::try_from(rect.height()).unwrap_or(0));
    let radius = LogicalPx::new(style.border_radius().map_or(0.0, BorderRadius::value));

    let background = bg_override.or_else(|| style.background());
    if let Some(color) = background {
        canvas.draw_rect(
            LogicalPx::new(left),
            LogicalPx::new(top),
            LogicalPx::new(width),
            LogicalPx::new(height),
            color.clone(),
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

    let radius = LogicalPx::new(style.border_radius().map_or(0.0, BorderRadius::value));
    let clamped = progress_val.clamp(0.0, 1.0);
    let left = f32::from(i16::try_from(rect.x()).unwrap_or(0));
    let top = f32::from(i16::try_from(rect.y()).unwrap_or(0));
    let width = f32::from(u16::try_from(rect.width()).unwrap_or(0));
    let height = f32::from(u16::try_from(rect.height()).unwrap_or(0));

    match orientation {
        Orientation::Horizontal => {
            let fill_w = (width * clamped).round();
            if fill_w > 0.0 {
                canvas.draw_rect(
                    LogicalPx::new(left),
                    LogicalPx::new(top),
                    LogicalPx::new(fill_w),
                    LogicalPx::new(height),
                    fill.clone(),
                    radius,
                );
            }
        }
        Orientation::Vertical => {
            let fill_h = (height * clamped).round();
            let fill_y = top + (height - fill_h);
            if fill_h > 0.0 {
                canvas.draw_rect(
                    LogicalPx::new(left),
                    LogicalPx::new(fill_y),
                    LogicalPx::new(width),
                    LogicalPx::new(fill_h),
                    fill.clone(),
                    radius,
                );
            }
        }
    }
}

fn render_text(
    canvas: &mut dyn Canvas,
    rect: &Rect,
    text: &crate::features::vdom::domain::TextContent,
    style: &ComputedStyle,
) {
    draw_bg_and_border(canvas, rect, style, None);
    let text_color = style.color().cloned().unwrap_or_else(|| {
        DrawingColor::Solid(crate::shared::primitives::color::Color::rgb(255, 255, 255))
    });
    canvas.draw_text(
        text.as_str(),
        style.font_family(),
        style.font_size(),
        text_color,
        text_origin(rect, style),
        text_alignment(rect, style),
    );
}

impl RenderNode {
    pub(crate) fn render_to_canvas(&self, canvas: &mut dyn Canvas) {
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
    use crate::shared::config::domain::{BorderRadius, BorderSize, FontFamily, FontSize};
    use crate::shared::primitives::BinaryData;
    use crate::shared::primitives::color::{Color, DrawingColor};
    use crate::shared::primitives::geometry::{Position, Rect, Size};
    use crate::shared::primitives::{TextAlign, TextAlignment};
    use crate::shared::rendering::ports::canvas::{CanvasCall, RecordingCanvas};

    #[test]
    fn test_render_to_canvas() {
        let mut canvas = RecordingCanvas::default();

        let mut rect_style = ComputedStyle::default();
        rect_style.set_background(DrawingColor::Solid(Color::rgb(255, 255, 255)));
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
        assert_eq!(canvas.kinds(), ["rect"]);

        let mut canvas = RecordingCanvas::default();
        let mut flex_style = ComputedStyle::default();
        flex_style.set_background(DrawingColor::Solid(Color::rgb(0, 0, 0)));
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
        assert_eq!(canvas.kinds(), ["rect"]);

        let mut canvas = RecordingCanvas::default();
        let mut text_style = ComputedStyle::default();
        text_style.set_color(DrawingColor::Solid(Color::rgb(0, 0, 0)));
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
        assert_eq!(canvas.kinds(), ["text"]);

        let mut canvas = RecordingCanvas::default();
        let mut prog_style = ComputedStyle::default();
        prog_style.set_background(DrawingColor::Solid(Color::rgb(0, 0, 0)));
        prog_style.set_accent_color(DrawingColor::Solid(Color::rgb(255, 0, 0)));
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
        assert_eq!(canvas.kinds(), ["rect", "rect"]);

        let mut canvas = RecordingCanvas::default();
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
        assert_eq!(canvas.kinds(), ["image"]);
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

    /// Where the one text request `canvas` received was drawn, and how aligned.
    fn drawn_text(canvas: &RecordingCanvas) -> (Position, TextAlignment) {
        match canvas.calls.as_slice() {
            [
                CanvasCall::Text {
                    position,
                    alignment,
                    ..
                },
            ] => (*position, *alignment),
            calls => panic!("expected exactly one text request, got {calls:?}"),
        }
    }

    #[test]
    fn test_text_is_drawn_inside_its_padding() {
        let mut padded = ComputedStyle::default();
        padded.set_padding(BoxMargin::new(4.0, 4.0, 2.0, 2.0));
        let mut canvas = RecordingCanvas::default();
        text_node(padded).render_to_canvas(&mut canvas);
        assert_eq!(drawn_text(&canvas).0, Position::new(22, 34));

        let mut canvas = RecordingCanvas::default();
        text_node(ComputedStyle::default()).render_to_canvas(&mut canvas);
        assert_eq!(drawn_text(&canvas).0, Position::new(20, 30));
    }

    #[test]
    fn test_text_alignment_carries_style_align_and_padded_width() {
        let mut centered = ComputedStyle::default();
        centered.set_padding(BoxMargin::new(4.0, 4.0, 2.0, 2.0));
        centered.set_text_align(TextAlign::Center);
        let mut canvas = RecordingCanvas::default();
        text_node(centered).render_to_canvas(&mut canvas);
        assert_eq!(
            drawn_text(&canvas).1,
            TextAlignment::new(TextAlign::Center, 26)
        );
    }

    #[test]
    fn test_text_alignment_defaults_to_left_over_full_box() {
        let mut canvas = RecordingCanvas::default();
        text_node(ComputedStyle::default()).render_to_canvas(&mut canvas);
        assert_eq!(
            drawn_text(&canvas).1,
            TextAlignment::new(TextAlign::Left, 30)
        );
    }

    fn solid(red: u8, green: u8, blue: u8) -> DrawingColor {
        DrawingColor::Solid(Color::rgb(red, green, blue))
    }

    fn progress_node(
        rect: Rect,
        value: f32,
        orientation: Orientation,
        style: ComputedStyle,
    ) -> RenderNode {
        RenderNode::Progress {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            rect,
            value: ProgressValue::new(value).unwrap(),
            orientation,
            style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        }
    }

    #[test]
    fn test_background_and_border_fill_and_stroke_the_node_rect() {
        let mut style = ComputedStyle::default();
        style.set_background(solid(10, 20, 30));
        style.set_border_color(solid(40, 50, 60));
        style.set_border_radius(BorderRadius::new(4.0));
        style.set_border_size(BorderSize::new(2.0));
        let node = RenderNode::Flex {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            rect: Rect::new(Position::new(3, 5), Size::new(40, 20)),
            children: vec![],
            style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        };
        let mut canvas = RecordingCanvas::default();
        node.render_to_canvas(&mut canvas);
        assert_eq!(
            canvas.calls,
            [
                CanvasCall::Rect {
                    area: [3.0, 5.0, 40.0, 20.0],
                    radius: 4.0,
                    color: solid(10, 20, 30),
                },
                CanvasCall::Border {
                    area: [3.0, 5.0, 40.0, 20.0],
                    radius: 4.0,
                    width: 2.0,
                    color: solid(40, 50, 60),
                },
            ]
        );
    }

    #[test]
    fn test_horizontal_progress_fills_from_the_left() {
        let mut style = ComputedStyle::default();
        style.set_accent_color(solid(200, 0, 0));
        style.set_border_radius(BorderRadius::new(3.0));
        let rect = Rect::new(Position::new(2, 4), Size::new(100, 10));
        let mut canvas = RecordingCanvas::default();
        progress_node(rect, 0.25, Orientation::Horizontal, style).render_to_canvas(&mut canvas);
        assert_eq!(
            canvas.calls,
            [CanvasCall::Rect {
                area: [2.0, 4.0, 25.0, 10.0],
                radius: 3.0,
                color: solid(200, 0, 0),
            }]
        );
    }

    #[test]
    fn test_vertical_progress_fills_from_the_bottom() {
        let mut style = ComputedStyle::default();
        style.set_color(solid(0, 200, 0));
        let rect = Rect::new(Position::new(2, 4), Size::new(10, 100));
        let mut canvas = RecordingCanvas::default();
        progress_node(rect, 0.25, Orientation::Vertical, style).render_to_canvas(&mut canvas);
        assert_eq!(
            canvas.calls,
            [CanvasCall::Rect {
                area: [2.0, 79.0, 10.0, 25.0],
                radius: 0.0,
                color: solid(0, 200, 0),
            }]
        );
    }

    #[test]
    fn test_progress_draws_no_fill_without_a_fill_color_or_value() {
        let mut background = ComputedStyle::default();
        background.set_background(solid(1, 2, 3));
        let rect = Rect::new(Position::new(0, 0), Size::new(100, 10));
        let mut canvas = RecordingCanvas::default();
        progress_node(rect, 0.5, Orientation::Horizontal, background).render_to_canvas(&mut canvas);
        assert_eq!(canvas.kinds(), ["rect"]);

        let mut accent = ComputedStyle::default();
        accent.set_accent_color(solid(200, 0, 0));
        for orientation in [Orientation::Horizontal, Orientation::Vertical] {
            let mut canvas = RecordingCanvas::default();
            progress_node(rect, 0.0, orientation, accent.clone()).render_to_canvas(&mut canvas);
            assert_eq!(canvas.calls, Vec::new());
        }
    }

    #[test]
    fn test_text_carries_its_style_font_and_color() {
        let mut style = ComputedStyle::default();
        style.set_color(solid(10, 20, 30));
        style.set_font_family(FontFamily::new("Inter".to_string()));
        style.set_font_size(FontSize::new(13.0));
        let mut canvas = RecordingCanvas::default();
        text_node(style).render_to_canvas(&mut canvas);
        assert_eq!(
            canvas.calls,
            [CanvasCall::Text {
                text: "7".to_string(),
                font_family: Some(FontFamily::new("Inter".to_string())),
                font_size: Some(FontSize::new(13.0)),
                color: solid(10, 20, 30),
                position: Position::new(20, 30),
                alignment: TextAlignment::new(TextAlign::Left, 30),
            }]
        );

        let mut canvas = RecordingCanvas::default();
        text_node(ComputedStyle::default()).render_to_canvas(&mut canvas);
        let [
            CanvasCall::Text {
                font_family,
                font_size,
                color,
                ..
            },
        ] = canvas.calls.as_slice()
        else {
            panic!("expected one text request, got {:?}", canvas.calls);
        };
        assert_eq!((font_family, font_size), (&None, &None));
        assert_eq!(color, &solid(255, 255, 255));
    }

    #[test]
    fn test_image_is_drawn_at_its_rect_from_its_pixels() {
        let image = RenderNode::Image {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            rect: Rect::new(Position::new(6, 8), Size::new(30, 20)),
            data: BinaryData::new(vec![0; 24]),
            pixel_size: Size::new(3, 2),
            tooltip: None,
            popup: None,
            panel: None,
        };
        let mut canvas = RecordingCanvas::default();
        image.render_to_canvas(&mut canvas);
        assert_eq!(
            canvas.calls,
            [CanvasCall::Image {
                pixel_size: Size::new(3, 2),
                logical_size: Size::new(30, 20),
                position: Position::new(6, 8),
            }]
        );
    }
}
