use super::support::{class_style, parse};
use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::layout_engine::domain::LayoutRoot;
use crate::features::layout_engine::domain::{AlignItems, RenderNode, StyledNode, TextMeasurer};
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::styling::adapters::fs_loader::CompositeStyleResolver;
use crate::features::styling::domain::{ClassNameList, CssLength, Orientation, ProgressValue};
use crate::features::vdom::domain::{
    ElementSelectors, ImageContent, Interactions, ProgressBar, TextContent, VNode,
};
use crate::shared::primitives::SizeConstraint;
use crate::shared::primitives::geometry::{Position, Size};
use crate::shared::rendering::ports::canvas::RecordingCanvas;
use crate::shared::rendering::ports::text::TextFont;

struct DummyMeasurer;
impl TextMeasurer for DummyMeasurer {
    fn measure(&mut self, text: &str, _font: TextFont<'_>) -> Size {
        let text_len = u32::try_from(text.len()).unwrap_or(0);
        Size::new(text_len.saturating_mul(8), 16)
    }
}

struct FixedDummyMeasurer;
impl TextMeasurer for FixedDummyMeasurer {
    fn measure(&mut self, _text: &str, _font: TextFont<'_>) -> Size {
        Size::new(10, 10)
    }
}

/// `styled` laid out by a fresh taffy engine at the origin with no size constraint.
fn lay_out(styled: StyledNode, measurer: &mut dyn TextMeasurer) -> RenderNode {
    TaffyLayoutAdapter::new()
        .calculate_layout(
            LayoutRoot::new(styled, Position::new(0, 0), SizeConstraint::none()),
            measurer,
        )
        .unwrap()
}

/// A theme under an arbitrary sheet name: a clock label and a battery progress bar.
const THEME_CSS: &str = r"
        .clock-label {
            color: #ff5555;
            font-size: 16px;
        }
        progress.battery {
            background-color: #282a36;
            accent-color: #50fa7b;
            border-radius: 4px;
        }
    ";

#[test]
fn test_arbitrary_style_name_and_progress_rendering() {
    let resolver = CompositeStyleResolver::new(vec![parse("random", THEME_CSS)]);

    let clock_node = VNode::new_text(
        TextContent::new("12:00".to_string()),
        ElementSelectors::new(Some(ClassNameList::parse("clock-label").unwrap()), None),
        Interactions::default(),
    );
    let styled_clock = clock_node.resolve_styles(&resolver, None);
    assert!((styled_clock.style().font_size().unwrap().value() - 16.0).abs() < f32::EPSILON);

    let h_prog = VNode::new_progress(
        ProgressBar::new(ProgressValue::new(0.6).unwrap(), Orientation::Horizontal),
        ElementSelectors::new(Some(ClassNameList::parse("battery").unwrap()), None),
        Interactions::default(),
    );
    let styled_h = h_prog.resolve_styles(&resolver, None);

    let render_h = lay_out(styled_h, &mut DummyMeasurer);
    let mut canvas = RecordingCanvas::default();
    render_h.render_to_canvas(&mut canvas);
    assert_eq!(canvas.kinds(), ["rect", "rect"]);
}

#[test]
fn test_width_height_parsing_and_layout() {
    let css = r"
        .icon {
            width: 20px;
            height: 20px;
        }
    ";
    let resolver = CompositeStyleResolver::new(vec![parse("systray", css)]);

    let img_node = VNode::new_image(
        ImageContent::new(vec![0; 400 * 4], Size::new(48, 48)),
        ElementSelectors::new(Some(ClassNameList::parse("icon").unwrap()), None),
        None,
    );

    let styled_img = img_node.resolve_styles(&resolver, None);
    assert_eq!(styled_img.style().width(), Some(CssLength::Pixels(20.0)));
    assert_eq!(styled_img.style().height(), Some(CssLength::Pixels(20.0)));

    let render_node = lay_out(styled_img, &mut FixedDummyMeasurer);

    assert_eq!(render_node.rect().width(), 20);
    assert_eq!(render_node.rect().height(), 20);
}

#[test]
fn test_advanced_css_properties_parsing_and_layout() {
    let css = r"
        .box {
            background: #1e1e2e;
            flex-grow: 1;
            flex-shrink: 0;
            align-self: center;
            padding-left: 12px;
            padding-right: 8px;
            border: 2px solid #7aa2f7;
        }
    ";
    let style = class_style(&*parse("box", css), "flex", "box");
    assert!(style.background().is_some());
    assert!((style.flex_grow().unwrap().value() - 1.0).abs() < f32::EPSILON);
    assert!((style.flex_shrink().unwrap().value() - 0.0).abs() < f32::EPSILON);
    assert_eq!(style.align_self(), Some(AlignItems::Center));
    assert!((style.padding().unwrap().left() - 12.0).abs() < f64::EPSILON);
    assert!((style.padding().unwrap().right() - 8.0).abs() < f64::EPSILON);
    assert!((style.border_size().unwrap().value() - 2.0).abs() < f32::EPSILON);
}
