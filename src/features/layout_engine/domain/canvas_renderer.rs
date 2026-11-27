//! Paints a laid-out `RenderNode` tree onto a canvas: backgrounds, borders, text, progress bars
//! and images.
use super::render_node::RenderNode;
use super::text_placement::{text_alignment, text_origin};
use crate::features::styling::domain::{ComputedStyle, Orientation};
use crate::shared::config::domain::BorderRadius;
use crate::shared::primitives::color::DrawingColor;
use crate::shared::primitives::geometry::{LogicalPx, Rect};
use crate::shared::rendering::ports::canvas::Canvas;
use crate::shared::rendering::ports::draw::{BorderStroke, RgbaImage, RoundedRect};
use crate::shared::rendering::ports::text::{TextFont, TextPlacement, TextRun};

/// A laid-out box and the computed style it is painted with.
#[derive(Debug, Clone, Copy)]
struct StyledBox<'a> {
    rect: &'a Rect,
    style: &'a ComputedStyle,
}

impl<'a> StyledBox<'a> {
    const fn new(rect: &'a Rect, style: &'a ComputedStyle) -> Self {
        Self { rect, style }
    }
}

/// How far a progress bar is filled, and in which direction the fill grows.
#[derive(Debug, Clone, Copy)]
struct ProgressFill {
    value: f32,
    orientation: Orientation,
}

fn draw_bg_and_border(
    canvas: &mut dyn Canvas,
    styled: StyledBox<'_>,
    bg_override: Option<&DrawingColor>,
) {
    let StyledBox { rect, style } = styled;
    let radius = LogicalPx::new(style.border_radius().map_or(0.0, BorderRadius::value));
    let shape = RoundedRect::new(*rect, radius);

    let background = bg_override.or_else(|| style.background());
    if let Some(color) = background {
        canvas.draw_rect(shape, color.clone());
    }
    if let (Some(size), Some(color)) = (style.border_size(), style.border_color()) {
        canvas.draw_border(
            shape,
            BorderStroke::new(color.clone(), LogicalPx::new(size.value())),
        );
    }
}

fn render_progress(canvas: &mut dyn Canvas, styled: StyledBox<'_>, fill: ProgressFill) {
    draw_bg_and_border(canvas, styled, None);
    let StyledBox { rect, style } = styled;
    let fill_color = style.accent_color().or_else(|| style.color());
    let Some(fill_paint) = fill_color else {
        return;
    };

    let radius = LogicalPx::new(style.border_radius().map_or(0.0, BorderRadius::value));
    let track = RoundedRect::new(*rect, radius);
    if let Some(filled) = filled_part(track, fill) {
        canvas.draw_rect(filled, fill_paint.clone());
    }
}

/// The part of `track` that `fill` covers, its value clamped to 0..=1 and rounded to whole
/// pixels: from the left when horizontal, from the bottom when vertical. `None` when it rounds to
/// nothing.
fn filled_part(track: RoundedRect, fill: ProgressFill) -> Option<RoundedRect> {
    let clamped = fill.value.clamp(0.0, 1.0);
    match fill.orientation {
        Orientation::Horizontal => {
            let fill_w = (track.width().value() * clamped).round();
            (fill_w > 0.0).then(|| track.with_width(LogicalPx::new(fill_w)))
        }
        Orientation::Vertical => {
            let fill_h = (track.height().value() * clamped).round();
            (fill_h > 0.0).then(|| track.with_bottom_height(LogicalPx::new(fill_h)))
        }
    }
}

fn render_text(
    canvas: &mut dyn Canvas,
    styled: StyledBox<'_>,
    text: &crate::features::vdom::domain::TextContent,
) {
    draw_bg_and_border(canvas, styled, None);
    let StyledBox { rect, style } = styled;
    let text_color = style.color().cloned().unwrap_or_else(|| {
        DrawingColor::Solid(crate::shared::primitives::color::Color::rgb(255, 255, 255))
    });
    canvas.draw_text(
        TextRun::new(
            text.as_str(),
            TextFont::new(style.font_family(), style.font_size()),
            text_color,
        ),
        TextPlacement::new(text_origin(rect, style), text_alignment(rect, style)),
    );
}

/// Paints a container's background and border, then each of its `children` over it, in order.
fn render_container(canvas: &mut dyn Canvas, styled: StyledBox<'_>, children: &[RenderNode]) {
    draw_bg_and_border(canvas, styled, None);
    for child in children {
        child.render_to_canvas(canvas);
    }
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
            } => render_container(canvas, StyledBox::new(rect, style), children),
            Self::Progress { .. } | Self::Text { .. } => self.render_content(canvas),
            Self::Rect { .. } | Self::Module { .. } | Self::Image { .. } => self.render_box(canvas),
        }
    }

    /// Paints a progress bar or a text node; nothing for any other node.
    fn render_content(&self, canvas: &mut dyn Canvas) {
        match self {
            Self::Progress {
                rect,
                value,
                orientation,
                style,
                ..
            } => {
                let fill = ProgressFill {
                    value: value.value(),
                    orientation: *orientation,
                };
                render_progress(canvas, StyledBox::new(rect, style), fill);
            }
            Self::Text {
                rect, text, style, ..
            } => render_text(canvas, StyledBox::new(rect, style), text),
            _ => {}
        }
    }

    /// Paints a rect (filled with its background, else its color), a module's host box or an
    /// image; nothing for any other node.
    fn render_box(&self, canvas: &mut dyn Canvas) {
        match self {
            Self::Rect { rect, style, .. } => {
                let fill = style.background().or_else(|| style.color());
                draw_bg_and_border(canvas, StyledBox::new(rect, style), fill);
            }
            Self::Module { rect, style, .. } => {
                draw_bg_and_border(canvas, StyledBox::new(rect, style), None);
            }
            Self::Image {
                rect,
                data,
                pixel_size,
                ..
            } => canvas.draw_image(RgbaImage::new(data, *pixel_size), *rect),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::render_node::RenderNode;
    use super::super::style::BoxMargin;
    use crate::features::styling::domain::{ComputedStyle, Orientation, ProgressValue};
    use crate::features::vdom::domain::{NodePath, SurfaceSpace, TextContent};
    use crate::shared::config::domain::{
        AxisMargins, BorderRadius, BorderSize, FontFamily, FontSize,
    };
    use crate::shared::primitives::BinaryData;
    use crate::shared::primitives::color::{Color, DrawingColor};
    use crate::shared::primitives::geometry::{Position, Rect, Size};
    use crate::shared::primitives::{ModuleKey, ModuleName, ModuleOptions};
    use crate::shared::primitives::{TextAlign, TextAlignment};
    use crate::shared::rendering::ports::canvas::{CanvasCall, RecordingCanvas};

    /// What painting `node` on a fresh canvas asked of it.
    fn painted(node: &RenderNode) -> RecordingCanvas {
        let mut canvas = RecordingCanvas::default();
        node.render_to_canvas(&mut canvas);
        canvas
    }

    /// A childless flex container laid out in `rect`.
    fn flex_node(rect: Rect, style: ComputedStyle) -> RenderNode {
        RenderNode::Flex {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            rect,
            children: vec![],
            style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        }
    }

    /// An image of `pixel_size` pixels, `data` holding them, laid out in `rect`.
    fn image_node(rect: Rect, data: Vec<u8>, pixel_size: Size) -> RenderNode {
        RenderNode::Image {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            rect,
            data: BinaryData::new(data),
            pixel_size,
            tooltip: None,
            popup: None,
            panel: None,
        }
    }

    /// An embedded module's host box.
    fn module_node(style: ComputedStyle) -> RenderNode {
        RenderNode::Module {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            rect: Rect::new(Position::new(0, 0), Size::new(10, 10)),
            key: ModuleKey::from_name(ModuleName::new("clock")),
            options: ModuleOptions::default(),
            style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        }
    }

    /// A container with a background holding one text child, painted below the child.
    fn parent_of_text() -> RenderNode {
        let origin_box = Rect::new(Position::new(0, 0), Size::new(10, 10));
        let mut parent = flex_node(
            origin_box,
            styled(|style| style.set_background(solid(1, 1, 1))),
        );
        if let RenderNode::Flex { children, .. } = &mut parent {
            children.push(text_node(styled(|style| style.set_color(solid(0, 0, 0)))));
        }
        parent
    }

    /// The default style changed by `change`.
    fn styled(change: impl FnOnce(&mut ComputedStyle)) -> ComputedStyle {
        let mut style = ComputedStyle::default();
        change(&mut style);
        style
    }

    #[test]
    fn test_render_to_canvas() {
        let origin_box = Rect::new(Position::new(0, 0), Size::new(10, 10));
        let progress = styled(|style| {
            style.set_background(solid(0, 0, 0));
            style.set_accent_color(solid(255, 0, 0));
        });
        let white = styled(|style| style.set_background(solid(255, 255, 255)));
        let dark_text = styled(|style| style.set_color(solid(0, 0, 0)));
        let cases: [(RenderNode, &[&str]); 4] = [
            (rect_node(white), &["rect"]),
            (text_node(dark_text), &["text"]),
            (
                progress_node(0.5, Orientation::Horizontal, progress),
                &["rect", "rect"],
            ),
            (
                image_node(origin_box, vec![0; 4], Size::new(1, 1)),
                &["image"],
            ),
        ];
        for (node, kinds) in cases {
            assert_eq!(painted(&node).kinds(), kinds);
        }
    }

    /// A container paints its box below its children, and a module host paints its box.
    #[test]
    fn test_boxes_paint_below_their_children() {
        let origin_box = Rect::new(Position::new(0, 0), Size::new(10, 10));
        let black = styled(|style| style.set_background(solid(0, 0, 0)));
        let module_host = styled(|style| style.set_background(solid(9, 9, 9)));
        let cases: [(RenderNode, &[&str]); 3] = [
            (flex_node(origin_box, black), &["rect"]),
            (module_node(module_host), &["rect"]),
            (parent_of_text(), &["rect", "text"]),
        ];
        for (node, kinds) in cases {
            assert_eq!(painted(&node).kinds(), kinds);
        }
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
        padded.set_padding(BoxMargin::new(
            AxisMargins::new(4.0, 4.0),
            AxisMargins::new(2.0, 2.0),
        ));
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
        centered.set_padding(BoxMargin::new(
            AxisMargins::new(4.0, 4.0),
            AxisMargins::new(2.0, 2.0),
        ));
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

    /// A progress bar at (2, 4), 100x10 when horizontal and 10x100 when vertical.
    fn progress_node(value: f32, orientation: Orientation, style: ComputedStyle) -> RenderNode {
        let size = match orientation {
            Orientation::Horizontal => Size::new(100, 10),
            Orientation::Vertical => Size::new(10, 100),
        };
        RenderNode::Progress {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            rect: Rect::new(Position::new(2, 4), size),
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
        let node = flex_node(Rect::new(Position::new(3, 5), Size::new(40, 20)), style);
        assert_eq!(
            painted(&node).calls,
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
        let mut canvas = RecordingCanvas::default();
        progress_node(0.25, Orientation::Horizontal, style).render_to_canvas(&mut canvas);
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
        let mut canvas = RecordingCanvas::default();
        progress_node(0.25, Orientation::Vertical, style).render_to_canvas(&mut canvas);
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
        let mut canvas = RecordingCanvas::default();
        progress_node(0.5, Orientation::Horizontal, background).render_to_canvas(&mut canvas);
        assert_eq!(canvas.kinds(), ["rect"]);

        let mut accent = ComputedStyle::default();
        accent.set_accent_color(solid(200, 0, 0));
        for orientation in [Orientation::Horizontal, Orientation::Vertical] {
            let mut canvas = RecordingCanvas::default();
            progress_node(0.0, orientation, accent.clone()).render_to_canvas(&mut canvas);
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
    }

    /// Text with no font or color of its own is drawn white in the default font.
    #[test]
    fn test_text_without_a_style_is_white_in_the_default_font() {
        let canvas = painted(&text_node(ComputedStyle::default()));
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
        let rect = Rect::new(Position::new(6, 8), Size::new(30, 20));
        let image = image_node(rect, vec![0; 24], Size::new(3, 2));
        assert_eq!(
            painted(&image).calls,
            [CanvasCall::Image {
                pixel_size: Size::new(3, 2),
                logical_size: Size::new(30, 20),
                position: Position::new(6, 8),
            }]
        );
    }

    fn rect_node(style: ComputedStyle) -> RenderNode {
        RenderNode::Rect {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            rect: Rect::new(Position::new(6, 2), Size::new(30, 12)),
            style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        }
    }

    #[test]
    fn test_rect_falls_back_to_its_color_when_it_has_no_background() {
        let mut colored = ComputedStyle::default();
        colored.set_color(solid(70, 80, 90));
        let mut canvas = RecordingCanvas::default();
        rect_node(colored.clone()).render_to_canvas(&mut canvas);
        assert_eq!(
            canvas.calls,
            [CanvasCall::Rect {
                area: [6.0, 2.0, 30.0, 12.0],
                radius: 0.0,
                color: solid(70, 80, 90),
            }]
        );

        colored.set_background(solid(5, 6, 7));
        let mut canvas = RecordingCanvas::default();
        rect_node(colored).render_to_canvas(&mut canvas);
        assert_eq!(
            canvas.calls,
            [CanvasCall::Rect {
                area: [6.0, 2.0, 30.0, 12.0],
                radius: 0.0,
                color: solid(5, 6, 7),
            }]
        );
    }

    #[test]
    fn test_text_paints_its_background_under_the_text() {
        let mut style = ComputedStyle::default();
        style.set_background(solid(9, 8, 7));
        let mut canvas = RecordingCanvas::default();
        text_node(style).render_to_canvas(&mut canvas);
        assert_eq!(canvas.kinds(), ["rect", "text"]);
    }
}
