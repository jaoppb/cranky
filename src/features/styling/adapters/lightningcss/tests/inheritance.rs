use crate::features::layout_engine::domain::StyledNode;
use crate::features::styling::adapters::fs_loader::CompositeStyleResolver;
use crate::features::styling::adapters::lightningcss::LightningCssAdapter;
use crate::features::styling::domain::{CssLength, StyleSheetName};
use crate::features::styling::ports::CssParserPort;
use crate::features::vdom::domain::{TextContent, VNode};
use crate::shared::primitives::color::DrawingColor;

#[test]
fn test_color_inherits_through_vdom_tree() {
    // The bug this stage fixes: `bar { color: ... }` in base.css never used
    // to reach a child text node, because resolution had no notion of
    // inheritance at all. Here `flex` stands in for the root (VNode's tag is
    // fixed per node kind, not customizable to "bar" in a unit test).
    let parser = LightningCssAdapter::new();
    let sheet = parser
        .parse_stylesheet(
            StyleSheetName::new("test").unwrap(),
            "flex { color: #7aa2f7; }",
        )
        .unwrap();
    let resolver = CompositeStyleResolver::new(vec![sheet]);

    let child = VNode::new_text(
        TextContent::new("hi".to_string()),
        None,
        None,
        None,
        None,
        None,
    );
    let root = VNode::new_flex(vec![child], None, None, None, None, None);

    let styled = root.resolve_styles(&resolver, None, None);
    let StyledNode::Flex { children, .. } = &styled else {
        panic!("expected a Flex root");
    };
    let StyledNode::Text { style, .. } = &children[0] else {
        panic!("expected a Text child");
    };
    assert_eq!(
        style.color(),
        Some(&DrawingColor::parse("#7aa2f7").unwrap())
    );
}

#[test]
fn test_own_declared_color_wins_over_inherited() {
    let parser = LightningCssAdapter::new();
    let sheet = parser
        .parse_stylesheet(
            StyleSheetName::new("test").unwrap(),
            "flex { color: #7aa2f7; } text { color: #ffffff; }",
        )
        .unwrap();
    let resolver = CompositeStyleResolver::new(vec![sheet]);

    let child = VNode::new_text(
        TextContent::new("hi".to_string()),
        None,
        None,
        None,
        None,
        None,
    );
    let root = VNode::new_flex(vec![child], None, None, None, None, None);

    let styled = root.resolve_styles(&resolver, None, None);
    let StyledNode::Flex { children, .. } = &styled else {
        panic!("expected a Flex root");
    };
    let StyledNode::Text { style, .. } = &children[0] else {
        panic!("expected a Text child");
    };
    assert_eq!(
        style.color(),
        Some(&DrawingColor::parse("#ffffff").unwrap())
    );
}

#[test]
fn test_em_resolves_through_vdom_tree_not_hardcoded_16px() {
    // flex's own font-size is 20px; text's em is relative to its *parent*
    // (flex's 20px), giving text a resolved font-size of 30px. text's own
    // width is then relative to *its own* resolved font-size (30px), not
    // flex's — 2em -> 60px, not the old hardcoded-16 guess of 32px.
    let parser = LightningCssAdapter::new();
    let sheet = parser
        .parse_stylesheet(
            StyleSheetName::new("test").unwrap(),
            "flex { font-size: 20px; } text { font-size: 1.5em; width: 2em; }",
        )
        .unwrap();
    let resolver = CompositeStyleResolver::new(vec![sheet]);

    let child = VNode::new_text(
        TextContent::new("hi".to_string()),
        None,
        None,
        None,
        None,
        None,
    );
    let root = VNode::new_flex(vec![child], None, None, None, None, None);

    let styled = root.resolve_styles(&resolver, None, None);
    let StyledNode::Flex { children, .. } = &styled else {
        panic!("expected a Flex root");
    };
    let StyledNode::Text { style, .. } = &children[0] else {
        panic!("expected a Text child");
    };
    assert_eq!(style.font_size().map(|fs| fs.value()), Some(30.0));
    assert_eq!(style.width(), Some(CssLength::Px(60.0)));
}
