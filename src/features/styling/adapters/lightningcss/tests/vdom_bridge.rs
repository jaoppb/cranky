//! The vdom styling bridge read through the real lightningcss resolver: what it tells the
//! cascade about a node's place among its siblings.
use super::support::parse;
use crate::features::layout_engine::domain::StyledNode;
use crate::features::styling::adapters::fs_loader::CompositeStyleResolver;
use crate::features::styling::domain::{ClassNameList, Opacity};
use crate::features::vdom::domain::{ElementSelectors, Interactions, VNode};

/// One opacity per sibling position: first, second and last.
const SIBLING_CSS: &str = ".x:first-child { opacity: 0.1; }
             .x:nth-child(2) { opacity: 0.5; }
             .x:last-child { opacity: 0.9; }";

/// The vdom bridge tells the cascade where each child sits among its siblings:
/// `:first-child`, `:nth-child(2)` and `:last-child` style the first, second
/// and third of three children.
#[test]
fn test_structural_pseudo_classes_follow_the_vnode_sibling_position() {
    let resolver = CompositeStyleResolver::new(vec![parse("test", SIBLING_CSS)]);
    let child = || {
        VNode::new_flex(
            Vec::new(),
            ElementSelectors::new(Some(ClassNameList::parse("x").unwrap()), None),
            Interactions::default(),
        )
    };
    let parent = VNode::new_flex(
        vec![child(), child(), child()],
        ElementSelectors::default(),
        Interactions::default(),
    );

    let styled = parent.resolve_styles(&resolver, None);

    let StyledNode::Flex { children, .. } = styled else {
        panic!("a flex resolves to a styled flex");
    };
    let opacities: Vec<_> = children
        .iter()
        .map(|child| child.style().opacity().map(Opacity::value))
        .collect();
    assert_eq!(opacities, vec![Some(0.1), Some(0.5), Some(0.9)]);
}
