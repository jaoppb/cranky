use super::support::{class_style, parse, resolve};
use crate::features::styling::adapters::lightningcss::LightningCssAdapter;
use crate::features::styling::domain::{
    ClassName, ElementId, ElementIdentity, ElementQuery, InheritedStyle, PseudoClass,
    StyleSheetName,
};
use crate::features::styling::ports::CssParserPort;
use crate::shared::primitives::color::{Color, DrawingColor};

/// The theme `test_parse_basic_css_rules` parses: a bar, a workspace button with its focus
/// and hover states, an element addressed by id and a progress bar.
const BASIC_CSS: &str = r"
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

#[test]
fn test_parse_basic_css_rules() {
    let parsed = parse("test", BASIC_CSS);

    // Test matching .workspace-btn
    let style_ws = class_style(&*parsed, "flex", "workspace-btn");
    assert!((style_ws.border_radius().unwrap().value() - 4.0).abs() < f32::EPSILON);
    assert!((style_ws.font_size().unwrap().value() - 14.0).abs() < f32::EPSILON);

    // Test matching .workspace-btn:hover
    let ws_class = ClassName::new("workspace-btn").unwrap();
    let ws_identity = ElementIdentity::new("flex", None, std::slice::from_ref(&ws_class));
    let style_ws_hover = resolve(
        &*parsed,
        &ElementQuery::new(ws_identity, &[PseudoClass::Hover], None),
    );
    let Some(DrawingColor::Solid(background)) = style_ws_hover.background() else {
        panic!("Expected background color #7aa2f7");
    };
    assert_eq!(*background, Color::rgb(122, 162, 247));

    // Test matching #hour-main
    let hour_id = ElementId::new("hour-main").unwrap();
    let hour_identity = ElementIdentity::new("text", Some(&hour_id), &[]);
    let style_hour = resolve(&*parsed, &ElementQuery::new(hour_identity, &[], None));
    let Some(DrawingColor::Solid(text_color)) = style_hour.color() else {
        panic!("Expected text color #c0caf5");
    };
    assert_eq!(*text_color, Color::rgb(192, 202, 245));

    // Test matching progress
    let progress_identity = ElementIdentity::new("progress", None, &[]);
    let style_prog = resolve(&*parsed, &ElementQuery::new(progress_identity, &[], None));
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

    let bar_parent = ElementQuery::new(ElementIdentity::new("bar", None, &[]), &[], None);

    let item_class = ClassName::new("item").unwrap();
    let item_query = ElementQuery::new(
        ElementIdentity::new("text", None, std::slice::from_ref(&item_class)),
        &[],
        Some(&bar_parent),
    );

    let (style, _) = parsed.resolve_style(&item_query, &InheritedStyle::default());
    assert!(style.color().is_some());
}

/// A bar with a solid border that a focused bar replaces with a gradient border image.
const GRADIENT_CSS: &str = r"
        bar {
            border-width: 2px;
            border-color: #565f89;
        }
        bar:focus {
            border-image-source: linear-gradient(45deg, #7aa2f7, #bb9af7);
        }
    ";

#[test]
fn test_gradient_border_color_resolution() {
    let parsed = parse("test", GRADIENT_CSS);
    let bar = ElementIdentity::new("bar", None, &[]);

    // 1. Unfocused bar -> Solid #565f89
    let style_unfocused = resolve(&*parsed, &ElementQuery::new(bar, &[], None));
    assert_eq!(
        style_unfocused.border_color(),
        Some(&DrawingColor::Solid(Color::rgb(86, 95, 137)))
    );

    // 2. Focused bar -> Gradient #7aa2f7 #bb9af7 45deg
    let style_focused = resolve(
        &*parsed,
        &ElementQuery::new(bar, &[PseudoClass::Focused], None),
    );
    let Some(DrawingColor::Gradient(colors, angle)) = style_focused.border_color() else {
        panic!("Expected gradient border color on focused bar");
    };
    assert_eq!(colors.len(), 2);
    assert_eq!(colors[0], Color::rgb(122, 162, 247));
    assert_eq!(colors[1], Color::rgb(187, 154, 247));
    assert!((angle - 45.0).abs() < f32::EPSILON);
}
