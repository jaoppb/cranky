use crate::features::styling::adapters::lightningcss::LightningCssAdapter;
use crate::features::styling::domain::{
    ClassName, ElementId, ElementQuery, InheritedStyle, PseudoClass, StyleSheetName,
};
use crate::features::styling::ports::CssParserPort;
use crate::shared::primitives::color::{Color, DrawingColor};

#[test]
fn test_parse_basic_css_rules() {
    let parser = LightningCssAdapter::new();
    let css = r"
        bar {
            background-color: #1a1b26;
            padding: 4px 8px;
            gap: 6px;
        }
        .workspace-btn {
            background-color: #3b4261;
            border-radius: 4px;
            font-size: 14px;
        }
        .workspace-btn:focus, .workspace-btn:hover {
            background-color: #7aa2f7;
        }
        #hour-main {
            color: #c0caf5;
        }
        progress {
            background-color: #24283b;
            border-radius: 6px;
            accent-color: #bb9af7;
        }
    ";

    let parsed = parser
        .parse_stylesheet(StyleSheetName::new("test").unwrap(), css)
        .expect("Failed to parse stylesheet");

    // Test matching .workspace-btn
    let ws_class = ClassName::new("workspace-btn").unwrap();
    let query_ws = ElementQuery::new("flex", None, std::slice::from_ref(&ws_class), &[], None);
    let style_ws = parsed.resolve_style(&query_ws, &InheritedStyle::default());
    assert!((style_ws.border_radius().unwrap().value() - 4.0).abs() < f32::EPSILON);
    assert!((style_ws.font_size().unwrap().value() - 14.0).abs() < f32::EPSILON);

    // Test matching .workspace-btn:hover
    let query_ws_hover = ElementQuery::new(
        "flex",
        None,
        std::slice::from_ref(&ws_class),
        &[PseudoClass::Hover],
        None,
    );
    let style_ws_hover = parsed.resolve_style(&query_ws_hover, &InheritedStyle::default());
    if let Some(DrawingColor::Solid(c)) = style_ws_hover.background() {
        assert_eq!(*c, Color::new(122, 162, 247, 255));
    } else {
        panic!("Expected background color #7aa2f7");
    }

    // Test matching #hour-main
    let hour_id = ElementId::new("hour-main").unwrap();
    let query_hour = ElementQuery::new("text", Some(&hour_id), &[], &[], None);
    let style_hour = parsed.resolve_style(&query_hour, &InheritedStyle::default());
    if let Some(DrawingColor::Solid(c)) = style_hour.color() {
        assert_eq!(*c, Color::new(192, 202, 245, 255));
    } else {
        panic!("Expected text color #c0caf5");
    }

    // Test matching progress
    let query_progress = ElementQuery::new("progress", None, &[], &[], None);
    let style_prog = parsed.resolve_style(&query_progress, &InheritedStyle::default());
    assert!((style_prog.border_radius().unwrap().value() - 6.0).abs() < f32::EPSILON);
    assert!(style_prog.accent_color().is_some());
}

#[test]
fn test_descendant_combinator() {
    let parser = LightningCssAdapter::new();
    let css = r"
        bar .item {
            color: #ffffff;
        }
    ";
    let parsed = parser
        .parse_stylesheet(StyleSheetName::new("test").unwrap(), css)
        .unwrap();

    let bar_parent = ElementQuery::new("bar", None, &[], &[], None);

    let item_class = ClassName::new("item").unwrap();
    let item_query = ElementQuery::new(
        "text",
        None,
        std::slice::from_ref(&item_class),
        &[],
        Some(&bar_parent),
    );

    let style = parsed.resolve_style(&item_query, &InheritedStyle::default());
    assert!(style.color().is_some());
}

#[test]
fn test_gradient_border_color_resolution() {
    let parser = LightningCssAdapter::new();
    let css = r"
        bar {
            border-width: 2px;
            border-color: #565f89;
        }
        bar:focus {
            border-color: #7aa2f7 #bb9af7 45deg;
        }
    ";
    let parsed = parser
        .parse_stylesheet(StyleSheetName::new("test").unwrap(), css)
        .unwrap();

    // 1. Unfocused bar -> Solid #565f89
    let query_unfocused = ElementQuery::new("bar", None, &[], &[], None);
    let style_unfocused = parsed.resolve_style(&query_unfocused, &InheritedStyle::default());
    assert_eq!(
        style_unfocused.border_color(),
        Some(&DrawingColor::Solid(Color::new(86, 95, 137, 255)))
    );

    // 2. Focused bar -> Gradient #7aa2f7 #bb9af7 45deg
    let query_focused = ElementQuery::new("bar", None, &[], &[PseudoClass::Focused], None);
    let style_focused = parsed.resolve_style(&query_focused, &InheritedStyle::default());
    if let Some(DrawingColor::Gradient(colors, angle)) = style_focused.border_color() {
        assert_eq!(colors.len(), 2);
        assert_eq!(colors[0], Color::new(122, 162, 247, 255));
        assert_eq!(colors[1], Color::new(187, 154, 247, 255));
        assert!((angle - 45.0).abs() < f32::EPSILON);
    } else {
        panic!("Expected gradient border color on focused bar");
    }
}
