use crate::features::layout_engine::domain::StyledNode;
use crate::features::styling::domain::{ElementQuery, InheritedStyle};
use crate::features::styling::ports::StyleResolverPort;
use crate::features::vdom::domain::{ElementSelectors, Interactions};
use crate::features::vdom::domain::{
    InteractedNodes, InteractionContext, NodePath, NodeRef, SurfaceSpace, TextContent, VNode,
};
use crate::shared::primitives::MonitorFocus;

struct MockResolver;
impl StyleResolverPort for MockResolver {
    fn resolve_style(
        &self,
        _query: &ElementQuery<'_>,
        _inherited: &InheritedStyle,
    ) -> (
        crate::features::styling::domain::ComputedStyle,
        InheritedStyle,
    ) {
        (
            crate::features::styling::domain::ComputedStyle::default(),
            InheritedStyle::default(),
        )
    }
}

fn text(content: &str) -> VNode {
    VNode::new_text(
        TextContent::new(content.to_string()),
        ElementSelectors::default(),
        Interactions::default(),
    )
}

/// A ref to the bar node at `indices`, with no keyed anchor.
fn bar_ref(indices: Vec<usize>) -> NodeRef {
    NodeRef::new(Vec::new(), NodePath::new(SurfaceSpace::Bar, indices))
}

#[test]
fn test_resolve_styles_interaction_propagation() {
    let root = VNode::new_flex(
        vec![text("c1"), text("c2")],
        ElementSelectors::default(),
        Interactions::default(),
    );

    let resolver = MockResolver;
    let interaction = InteractionContext::new(
        InteractedNodes::new(
            Some(bar_ref(vec![0])),
            Some(bar_ref(vec![0])),
            Some(bar_ref(vec![1])),
        ),
        MonitorFocus::Focused,
    );

    let styled = root.resolve_styles(&resolver, Some(&interaction));
    assert_eq!(styled.path(), &NodePath::root_in(SurfaceSpace::Bar));

    let StyledNode::Flex { children, .. } = styled else {
        panic!("Expected StyledNode::Flex");
    };
    let paths: Vec<&NodePath> = children.iter().map(StyledNode::path).collect();
    let expected = [vec![0], vec![1]].map(|indices| NodePath::new(SurfaceSpace::Bar, indices));
    assert_eq!(paths, expected.iter().collect::<Vec<_>>());
}

#[test]
fn test_vnode_with_popup_and_style_resolution() {
    let popup_content = VNode::new_text(
        TextContent::new("popup body".to_string()),
        ElementSelectors::default(),
        Interactions::default(),
    );
    let button = VNode::new_text(
        TextContent::new("Open".to_string()),
        ElementSelectors::default(),
        Interactions::default(),
    )
    .with_popup(crate::features::vdom::domain::PopupSpec::new(Box::new(
        popup_content.clone(),
    )));

    assert!(button.popup().is_some());
    assert_eq!(button.popup().unwrap().content(), &popup_content);

    let resolver = MockResolver;
    let styled = button.resolve_styles(&resolver, None);
    assert!(styled.popup().is_some());
}

#[test]
fn test_vnode_grid_creation_and_resolution() {
    use crate::features::styling::domain::{DisplayMode, GridAutoFlow};
    let grid_node = VNode::new_grid(
        vec![text("Item 1"), text("Item 2")],
        ElementSelectors::default(),
        Interactions::default(),
    );
    let tag = grid_node.tag();
    let shape = (tag, tag.is_container(), grid_node.children().len());
    assert_eq!(
        shape,
        (crate::features::vdom::domain::NodeTag::Grid, true, 2)
    );

    let resolver = MockResolver;
    let styled = grid_node.resolve_styles(&resolver, None);
    let StyledNode::Grid {
        children, style, ..
    } = styled
    else {
        panic!("Expected StyledNode::Grid");
    };
    let layout = (children.len(), style.display(), style.grid_auto_flow());
    let expected = (2, Some(DisplayMode::Grid), Some(GridAutoFlow::Row));
    assert_eq!(layout, expected);
}
