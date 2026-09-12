use crate::features::layout_engine::domain::StyledNode;
use crate::features::styling::domain::{ElementQuery, InheritedStyle};
use crate::features::styling::ports::StyleResolverPort;
use crate::features::vdom::domain::{InteractionContext, NodePath, TextContent, VNode};

struct MockResolver;
impl StyleResolverPort for MockResolver {
    fn resolve_style(
        &self,
        _query: &ElementQuery,
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

#[test]
fn test_resolve_styles_interaction_propagation() {
    let child1 = VNode::new_text(
        TextContent::new("c1".to_string()),
        None,
        None,
        None,
        None,
        None,
    );
    let child2 = VNode::new_text(
        TextContent::new("c2".to_string()),
        None,
        None,
        None,
        None,
        None,
    );
    let root = VNode::new_flex(vec![child1, child2], None, None, None, None, None);

    let resolver = MockResolver;
    let interaction = InteractionContext::new(
        Some(NodePath::new(vec![0])),
        Some(NodePath::new(vec![0])),
        Some(NodePath::new(vec![1])),
        true,
    );

    let styled = root.resolve_styles(&resolver, Some(&interaction), None);
    assert_eq!(styled.path(), &NodePath::root());

    if let StyledNode::Flex { children, .. } = styled {
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].path(), &NodePath::new(vec![0]));
        assert_eq!(children[1].path(), &NodePath::new(vec![1]));
    } else {
        panic!("Expected StyledNode::Flex");
    }
}

#[test]
fn test_vnode_with_popup_and_style_resolution() {
    let popup_content = VNode::new_text(
        TextContent::new("popup body".to_string()),
        None,
        None,
        None,
        None,
        None,
    );
    let button = VNode::new_text(
        TextContent::new("Open".to_string()),
        None,
        None,
        None,
        None,
        None,
    )
    .with_popup(crate::features::vdom::domain::PopupSpec::new(Box::new(
        popup_content.clone(),
    )));

    assert!(button.popup().is_some());
    assert_eq!(button.popup().unwrap().content(), &popup_content);

    let resolver = MockResolver;
    let styled = button.resolve_styles(&resolver, None, None);
    assert!(styled.popup().is_some());
}

#[test]
fn test_vnode_grid_creation_and_resolution() {
    let child1 = VNode::new_text(
        TextContent::new("Item 1".to_string()),
        None,
        None,
        None,
        None,
        None,
    );
    let child2 = VNode::new_text(
        TextContent::new("Item 2".to_string()),
        None,
        None,
        None,
        None,
        None,
    );

    let grid_node = VNode::new_grid(vec![child1, child2], None, None, None, None, None);
    assert_eq!(
        grid_node.tag(),
        crate::features::vdom::domain::NodeTag::Grid
    );
    assert!(grid_node.tag().is_container());
    assert_eq!(grid_node.children().len(), 2);

    let resolver = MockResolver;
    let styled = grid_node.resolve_styles(&resolver, None, None);
    if let StyledNode::Grid {
        children, style, ..
    } = styled
    {
        assert_eq!(children.len(), 2);
        assert_eq!(
            style.display(),
            Some(crate::features::styling::domain::DisplayMode::Grid)
        );
        assert_eq!(
            style.grid_auto_flow(),
            Some(crate::features::styling::domain::GridAutoFlow::Row)
        );
    } else {
        panic!("Expected StyledNode::Grid");
    }
}
