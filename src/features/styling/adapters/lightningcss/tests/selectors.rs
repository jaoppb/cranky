use super::support::{class_style, parse, resolve};
use crate::features::layout_engine::domain::BoxMargin;
use crate::features::styling::adapters::fs_loader::CompositeStyleResolver;
use crate::features::styling::adapters::lightningcss::LightningCssAdapter;
use crate::features::styling::domain::ComputedStyle;
use crate::features::styling::domain::{
    ClassName, ClassNameList, ElementIdentity, ElementQuery, Emptiness, InheritedStyle, Opacity,
    SiblingPosition, StyleSheetName,
};
use crate::features::styling::ports::{CssParserPort, ParsedStyleSheetPort};
use crate::features::vdom::domain::{ElementSelectors, Interactions, VNode};
use crate::shared::config::domain::BorderRadius;
use crate::shared::primitives::color::{Color, DrawingColor};

/// The sheet the structural pseudo-class tests read: one `.item` rule per pseudo-class.
const STRUCTURAL_CSS: &str = r"
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

/// The style `sheet` gives a `.item` flex at `position` among its siblings, with `emptiness`.
fn item_style(
    sheet: &dyn ParsedStyleSheetPort,
    position: SiblingPosition,
    emptiness: Emptiness,
) -> ComputedStyle {
    let class_item = ClassName::new("item").unwrap();
    let identity = ElementIdentity::new("flex", None, std::slice::from_ref(&class_item));
    let query = ElementQuery::new(identity, &[], None).with_structural_context(position, emptiness);
    resolve(sheet, &query)
}

#[test]
fn test_structural_pseudo_classes() {
    let parsed = parse("test", STRUCTURAL_CSS);

    // 1. First child in a list of 3 (index 0, total 3)
    let first = SiblingPosition::new(0, 3);
    let style_first = item_style(&*parsed, first, Emptiness::NonEmpty);
    assert_eq!(style_first.padding().map(BoxMargin::left), Some(10.0));
    assert_eq!(style_first.padding().map(BoxMargin::right), Some(0.0));
    assert!(style_first.background().is_some());
    // Odd: 1st is 2n+1 -> #111111
    assert_eq!(
        style_first.background(),
        Some(&DrawingColor::Solid(Color::rgb(17, 17, 17)))
    );

    // 2. Second child (index 1, total 3)
    let second = SiblingPosition::new(1, 3);
    let style_second = item_style(&*parsed, second, Emptiness::NonEmpty);
    assert!(style_second.padding().is_none());
    // Even: 2nd is 2n -> #222222
    assert_eq!(
        style_second.background(),
        Some(&DrawingColor::Solid(Color::rgb(34, 34, 34)))
    );

    // 3. Last child (index 2, total 3)
    let last = SiblingPosition::new(2, 3);
    let style_last = item_style(&*parsed, last, Emptiness::NonEmpty);
    assert_eq!(style_last.padding().map(BoxMargin::right), Some(15.0));
}

/// The same sheet read on a lone child: `:only-child` applies, and `:empty` applies only
/// when the child has no content.
#[test]
fn test_only_child_and_empty_pseudo_classes() {
    let parsed = parse("test", STRUCTURAL_CSS);

    // 4. Only child (index 0, total 1)
    let only = SiblingPosition::new(0, 1);
    let style_only = item_style(&*parsed, only, Emptiness::NonEmpty);
    assert_eq!(
        style_only.border_radius().map(BorderRadius::value),
        Some(8.0)
    );

    // 5. Empty
    let style_empty = item_style(&*parsed, only, Emptiness::Empty);
    assert!((style_empty.opacity().unwrap().value() - 0.5).abs() < f32::EPSILON);
}

/// The same sheet read on an element outside any sibling list: `:not(.active)` styles an
/// item without the `active` class and skips one with it.
#[test]
fn test_not_pseudo_class_skips_the_excluded_class() {
    let parsed = parse("test", STRUCTURAL_CSS);

    // 6. Not active vs Active
    let style_not_active = class_style(&*parsed, "flex", "item");
    assert!(style_not_active.color().is_some());

    let class_item = ClassName::new("item").unwrap();
    let class_active = ClassName::new("active").unwrap();
    let active_classes = [class_item, class_active];
    let query_active = ElementQuery::new(
        ElementIdentity::new("flex", None, &active_classes),
        &[],
        None,
    );
    let style_active = resolve(&*parsed, &query_active);
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
            ElementSelectors::new(Some(ClassNameList::parse("x").unwrap()), None),
            Interactions::default(),
        );
        node.resolve_styles(&resolver, None)
            .style()
            .opacity()
            .map(Opacity::value)
    };
    assert_eq!(opacity(Vec::new()), Some(0.5));
    let child = VNode::new_rect(ElementSelectors::default(), Interactions::default());
    assert_eq!(opacity(vec![child]), None);
}
