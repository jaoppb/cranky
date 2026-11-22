use crate::features::layout_engine::domain::BoxMargin;
use crate::features::styling::adapters::fs_loader::CompositeStyleResolver;
use crate::features::styling::adapters::lightningcss::LightningCssAdapter;
use crate::features::styling::domain::{
    ClassName, ClassNameList, ElementIdentity, ElementQuery, Emptiness, InheritedStyle, Opacity,
    SiblingPosition, StyleSheetName,
};
use crate::features::styling::ports::CssParserPort;
use crate::features::vdom::domain::VNode;
use crate::shared::config::domain::BorderRadius;
use crate::shared::primitives::color::{Color, DrawingColor};

#[test]
fn test_structural_pseudo_classes() {
    let parser = LightningCssAdapter::new();
    let css = r"
        .item:first-child {
            padding-left: 10px;
        }
        .item:last-child {
            padding-right: 15px;
        }
        .item:only-child {
            border-radius: 8px;
        }
        .item:nth-child(2n+1) {
            background-color: #111111;
        }
        .item:nth-child(2n) {
            background-color: #222222;
        }
        .item:empty {
            opacity: 0.5;
        }
        .item:not(.active) {
            color: #888888;
        }
    ";
    let parsed = parser
        .parse_stylesheet(StyleSheetName::new("test").unwrap(), css)
        .unwrap();

    let class_item = ClassName::new("item").unwrap();
    let class_active = ClassName::new("active").unwrap();

    // 1. First child in a list of 3 (index 0, total 3)
    let query_first = ElementQuery::new(
        ElementIdentity::new("flex", None, std::slice::from_ref(&class_item)),
        &[],
        None,
    )
    .with_structural_context(SiblingPosition::new(0, 3), Emptiness::NonEmpty);
    let (style_first, _) = parsed.resolve_style(&query_first, &InheritedStyle::default());
    assert_eq!(style_first.padding().map(BoxMargin::left), Some(10.0));
    assert_eq!(style_first.padding().map(BoxMargin::right), Some(0.0));
    assert!(style_first.background().is_some());
    // Odd: 1st is 2n+1 -> #111111
    if let Some(DrawingColor::Solid(background)) = style_first.background() {
        assert_eq!(*background, Color::rgb(17, 17, 17));
    }

    // 2. Second child (index 1, total 3)
    let query_second = ElementQuery::new(
        ElementIdentity::new("flex", None, std::slice::from_ref(&class_item)),
        &[],
        None,
    )
    .with_structural_context(SiblingPosition::new(1, 3), Emptiness::NonEmpty);
    let (style_second, _) = parsed.resolve_style(&query_second, &InheritedStyle::default());
    assert!(style_second.padding().is_none());
    // Even: 2nd is 2n -> #222222
    if let Some(DrawingColor::Solid(background)) = style_second.background() {
        assert_eq!(*background, Color::rgb(34, 34, 34));
    }

    // 3. Last child (index 2, total 3)
    let query_last = ElementQuery::new(
        ElementIdentity::new("flex", None, std::slice::from_ref(&class_item)),
        &[],
        None,
    )
    .with_structural_context(SiblingPosition::new(2, 3), Emptiness::NonEmpty);
    let (style_last, _) = parsed.resolve_style(&query_last, &InheritedStyle::default());
    assert_eq!(style_last.padding().map(BoxMargin::right), Some(15.0));

    // 4. Only child (index 0, total 1)
    let query_only = ElementQuery::new(
        ElementIdentity::new("flex", None, std::slice::from_ref(&class_item)),
        &[],
        None,
    )
    .with_structural_context(SiblingPosition::new(0, 1), Emptiness::NonEmpty);
    let (style_only, _) = parsed.resolve_style(&query_only, &InheritedStyle::default());
    assert_eq!(
        style_only.border_radius().map(BorderRadius::value),
        Some(8.0)
    );

    // 5. Empty
    let query_empty = ElementQuery::new(
        ElementIdentity::new("flex", None, std::slice::from_ref(&class_item)),
        &[],
        None,
    )
    .with_structural_context(SiblingPosition::new(0, 1), Emptiness::Empty);
    let (style_empty, _) = parsed.resolve_style(&query_empty, &InheritedStyle::default());
    assert!((style_empty.opacity().unwrap().value() - 0.5).abs() < f32::EPSILON);

    // 6. Not active vs Active
    let query_not_active = ElementQuery::new(
        ElementIdentity::new("flex", None, std::slice::from_ref(&class_item)),
        &[],
        None,
    );
    let (style_not_active, _) = parsed.resolve_style(&query_not_active, &InheritedStyle::default());
    assert!(style_not_active.color().is_some());

    let active_classes = [class_item.clone(), class_active];
    let query_active = ElementQuery::new(
        ElementIdentity::new("flex", None, &active_classes),
        &[],
        None,
    );
    let (style_active, _) = parsed.resolve_style(&query_active, &InheritedStyle::default());
    assert!(style_active.color().is_none());
}

#[test]
fn test_empty_pseudo_class_skips_an_element_with_content() {
    let parsed = LightningCssAdapter::new()
        .parse_stylesheet(
            StyleSheetName::new("test").unwrap(),
            ".item:empty { opacity: 0.5; }",
        )
        .unwrap();
    let class_item = ClassName::new("item").unwrap();
    let opacity = |emptiness| {
        let query = ElementQuery::new(
            ElementIdentity::new("flex", None, std::slice::from_ref(&class_item)),
            &[],
            None,
        )
        .with_structural_context(SiblingPosition::new(0, 1), emptiness);
        let (style, _) = parsed.resolve_style(&query, &InheritedStyle::default());
        style.opacity().map(Opacity::value)
    };
    assert_eq!(opacity(Emptiness::Empty), Some(0.5));
    assert_eq!(opacity(Emptiness::NonEmpty), None);
}

/// The vdom bridge tells the cascade whether a node is empty: `:empty`
/// styles a flex with no children and skips one with a child.
#[test]
fn test_empty_pseudo_class_follows_the_vnode_children() {
    let sheet = LightningCssAdapter::new()
        .parse_stylesheet(
            StyleSheetName::new("test").unwrap(),
            ".x:empty { opacity: 0.5; }",
        )
        .unwrap();
    let resolver = CompositeStyleResolver::new(vec![sheet]);
    let opacity = |children: Vec<VNode>| {
        let node = VNode::new_flex(
            children,
            Some(ClassNameList::parse("x").unwrap()),
            None,
            None,
            None,
            None,
        );
        node.resolve_styles(&resolver, None)
            .style()
            .opacity()
            .map(Opacity::value)
    };
    assert_eq!(opacity(Vec::new()), Some(0.5));
    let child = VNode::new_rect(None, None, None, None, None);
    assert_eq!(opacity(vec![child]), None);
}
