use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::layout_engine::domain::{StyledNode, TextMeasurer};
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::styling::adapters::fs_loader::CompositeStyleResolver;
use crate::features::styling::adapters::lightningcss::LightningCssAdapter;
use crate::features::styling::domain::{ClassNameList, StyleSheetName};
use crate::features::styling::ports::CssParserPort;
use crate::features::vdom::domain::{TextContent, VNode};
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::SizeConstraint;
use crate::shared::primitives::geometry::{Position, Size};
use crate::shared::primitives::text_align::{TextAlign, TextAlignment};
use crate::shared::rendering::ports::canvas::MockCanvas;
use std::sync::{Arc, Mutex};

fn resolver(css: &str) -> CompositeStyleResolver {
    let sheet = LightningCssAdapter::new()
        .parse_stylesheet(StyleSheetName::new("test").unwrap(), css)
        .unwrap();
    CompositeStyleResolver::new(vec![sheet])
}

fn text(content: &str, class: Option<&str>) -> VNode {
    VNode::new_text(
        TextContent::new(content.to_string()),
        class.map(|c| ClassNameList::parse(c).unwrap()),
        None,
        None,
        None,
        None,
    )
}

/// Resolves `css` against a flex parent (optionally classed) holding one
/// text child, returning the child's `text-align`.
fn child_align(
    css: &str,
    parent_class: Option<&str>,
    child_class: Option<&str>,
) -> Option<TextAlign> {
    let root = VNode::new_flex(
        vec![text("hi", child_class)],
        parent_class.map(|c| ClassNameList::parse(c).unwrap()),
        None,
        None,
        None,
        None,
    );
    let StyledNode::Flex { children, .. } = root.resolve_styles(&resolver(css), None, None) else {
        panic!("expected a Flex root");
    };
    children.first().and_then(|c| c.style().text_align())
}

#[test]
fn test_text_align_keywords_parse() {
    for (keyword, expected) in [
        ("left", TextAlign::Left),
        ("center", TextAlign::Center),
        ("right", TextAlign::Right),
    ] {
        let css = format!("text {{ text-align: {keyword}; }}");
        assert_eq!(child_align(&css, None, None), Some(expected), "{keyword}");
    }
}

#[test]
fn test_unsupported_text_align_keywords_stay_undeclared() {
    for keyword in ["justify", "start", "end"] {
        let css = format!("text {{ text-align: {keyword}; }}");
        assert_eq!(child_align(&css, None, None), None, "{keyword}");
    }
}

#[test]
fn test_text_align_inherits_from_parent() {
    let css = "flex { text-align: center; }";
    assert_eq!(child_align(css, None, None), Some(TextAlign::Center));
}

#[test]
fn test_own_text_align_wins_over_inherited() {
    let css = "flex { text-align: center; } text { text-align: right; }";
    assert_eq!(child_align(css, None, None), Some(TextAlign::Right));
}

#[test]
fn test_text_align_initial_resets_inheritance() {
    let css = "flex { text-align: center; } text { text-align: initial; }";
    assert_eq!(child_align(css, None, None), None);
}

#[test]
fn test_text_align_inherit_keyword_pulls_parent_value() {
    let css = "flex { text-align: right; } text { text-align: inherit; }";
    assert_eq!(child_align(css, None, None), Some(TextAlign::Right));
}

struct EightPerChar;
impl TextMeasurer for EightPerChar {
    fn measure(&mut self, text: &str, _f: Option<&FontFamily>, _s: Option<FontSize>) -> Size {
        Size::new(u32::try_from(text.len()).unwrap_or(0).saturating_mul(8), 16)
    }
}

/// Lays out a calendar-shaped tree under the shipped `calendar.css` and
/// returns every `draw_text` call as `(text, alignment)` in paint order.
fn paint_calendar() -> Vec<(String, TextAlignment)> {
    let class = |c: &str| Some(ClassNameList::parse(c).unwrap());
    let header = VNode::new_flex(
        vec![
            text("<", Some("nav-btn")),
            text("May 2026", Some("title")),
            text(">", Some("nav-btn")),
        ],
        class("header"),
        None,
        None,
        None,
        None,
    );
    let days = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"]
        .iter()
        .map(|d| text(d, Some("weekday")))
        .collect();
    let weekdays = VNode::new_grid(days, class("weekdays"), None, None, None, None);
    let cells = (1..=7).map(|d| text(&d.to_string(), Some("day"))).collect();
    let grid = VNode::new_grid(cells, class("grid"), None, None, None, None);
    let root = VNode::new_flex(
        vec![header, weekdays, grid],
        class("root"),
        None,
        None,
        None,
        None,
    );

    let resolver = resolver(include_str!("../../../../../../assets/styles/calendar.css"));
    let styled = root.resolve_styles(&resolver, None, None);
    let laid_out = TaffyLayoutAdapter::new()
        .calculate_layout_with_constraints(
            styled,
            &mut EightPerChar,
            Position::new(0, 0),
            SizeConstraint::new(Some(280), None),
        )
        .unwrap();

    let drawn = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&drawn);
    let mut canvas = MockCanvas::new();
    canvas.expect_draw_rect().returning(|_, _, _, _, _, _| ());
    canvas.expect_draw_border().returning(|_, _, _, _, _| ());
    canvas
        .expect_draw_text()
        .returning(move |t, _, _, _, _, alignment| {
            sink.lock().unwrap().push((t.to_string(), alignment));
        });
    laid_out.render_to_canvas(&mut canvas);
    drawn.lock().unwrap().clone()
}

#[test]
fn test_calendar_title_is_centered_across_the_space_between_nav_buttons() {
    let calls = paint_calendar();
    let (_, title) = calls.iter().find(|(t, _)| t == "May 2026").unwrap();
    // 280 host - 2*12 padding = 256 header; each nav button is
    // 8 (glyph) + 2*6 padding = 20 wide, so `flex-grow: 1` must hand the
    // title the remaining 216, not just its own 64-wide glyph run.
    assert_eq!(*title, TextAlignment::new(TextAlign::Center, 216));
}

#[test]
fn test_calendar_day_and_weekday_cells_are_centered_in_their_column() {
    let calls = paint_calendar();
    for label in ["Su", "Sa", "1", "7"] {
        let (_, alignment) = calls.iter().find(|(t, _)| t == label).unwrap();
        assert_eq!(alignment.align(), TextAlign::Center, "{label}");
        // 7 columns, 4px gaps across 254: ~32px wide (days lose 4px padding),
        // far more than the 16px glyph run a content-sized box would give.
        assert!(alignment.available_width() >= 28, "{label}: {alignment:?}");
    }
}
