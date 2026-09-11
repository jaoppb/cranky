use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::layout_engine::domain::{AlignItems, TextMeasurer};
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::styling::adapters::fs_loader::CompositeStyleResolver;
use crate::features::styling::adapters::lightningcss::LightningCssAdapter;
use crate::features::styling::domain::{
    ClassName, ClassNameList, CssLength, ElementQuery, InheritedStyle, Orientation, ProgressValue,
    StyleSheetName,
};
use crate::features::styling::ports::CssParserPort;
use crate::features::vdom::domain::{TextContent, VNode};
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::geometry::{Position, Size};
use crate::shared::rendering::ports::canvas::MockCanvas;

struct DummyMeasurer;
impl TextMeasurer for DummyMeasurer {
    fn measure(&mut self, text: &str, _f: Option<&FontFamily>, _s: Option<FontSize>) -> Size {
        let text_len = u32::try_from(text.len()).unwrap_or(0);
        Size::new(text_len.saturating_mul(8), 16)
    }
}

struct FixedDummyMeasurer;
impl TextMeasurer for FixedDummyMeasurer {
    fn measure(&mut self, _text: &str, _f: Option<&FontFamily>, _s: Option<FontSize>) -> Size {
        Size::new(10, 10)
    }
}

#[test]
fn test_arbitrary_style_name_and_progress_rendering() {
    let parser = LightningCssAdapter::new();
    let theme_css = r"
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

    let parsed = parser
        .parse_stylesheet(StyleSheetName::new("random").unwrap(), theme_css)
        .unwrap();
    let resolver = CompositeStyleResolver::new(vec![parsed]);

    let clock_node = VNode::new_text(
        TextContent::new("12:00".to_string()),
        Some(ClassNameList::parse("clock-label").unwrap()),
        None,
        None,
        None,
        None,
    );
    let styled_clock = clock_node.resolve_styles(&resolver, None, None);
    assert!((styled_clock.style().font_size().unwrap().value() - 16.0).abs() < f32::EPSILON);

    let h_prog = VNode::new_progress(
        ProgressValue::new(0.6).unwrap(),
        Orientation::Horizontal,
        Some(ClassNameList::parse("battery").unwrap()),
        None,
        None,
        None,
        None,
    );
    let styled_h = h_prog.resolve_styles(&resolver, None, None);
    let mut engine = TaffyLayoutAdapter::new();

    let render_h = engine
        .calculate_layout(styled_h, &mut DummyMeasurer, Position::new(0, 0))
        .unwrap();
    let mut mock_canvas = MockCanvas::new();
    mock_canvas.expect_draw_rect().times(2).return_const(());
    render_h.render_to_canvas(&mut mock_canvas);
}

#[test]
fn test_width_height_parsing_and_layout() {
    let parser = LightningCssAdapter::new();
    let css = r"
        .icon {
            width: 20px;
            height: 20px;
        }
    ";
    let parsed = parser
        .parse_stylesheet(StyleSheetName::new("systray").unwrap(), css)
        .unwrap();
    let resolver = CompositeStyleResolver::new(vec![parsed]);

    let img_node = VNode::new_image(
        vec![0; 400 * 4],
        Size::new(48, 48),
        Some(ClassNameList::parse("icon").unwrap()),
        None,
        None,
    );

    let styled_img = img_node.resolve_styles(&resolver, None, None);
    assert_eq!(styled_img.style().width(), Some(CssLength::Px(20.0)));
    assert_eq!(styled_img.style().height(), Some(CssLength::Px(20.0)));

    let mut engine = TaffyLayoutAdapter::new();
    let render_node = engine
        .calculate_layout(styled_img, &mut FixedDummyMeasurer, Position::new(0, 0))
        .unwrap();

    assert_eq!(render_node.rect().width(), 20);
    assert_eq!(render_node.rect().height(), 20);
}

#[test]
fn test_advanced_css_properties_parsing_and_layout() {
    let parser = LightningCssAdapter::new();
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
    let parsed = parser
        .parse_stylesheet(StyleSheetName::new("box").unwrap(), css)
        .unwrap();

    let class = ClassName::new("box").unwrap();
    let query = ElementQuery::new("flex", None, std::slice::from_ref(&class), &[], None);

    let style = parsed.resolve_style(&query, &InheritedStyle::default());
    assert!(style.background().is_some());
    assert!((style.flex_grow().unwrap().value() - 1.0).abs() < f32::EPSILON);
    assert!((style.flex_shrink().unwrap().value() - 0.0).abs() < f32::EPSILON);
    assert_eq!(style.align_self(), Some(AlignItems::Center));
    assert!((style.padding().unwrap().left() - 12.0).abs() < f64::EPSILON);
    assert!((style.padding().unwrap().right() - 8.0).abs() < f64::EPSILON);
    assert!((style.border_size().unwrap().value() - 2.0).abs() < f32::EPSILON);
}
