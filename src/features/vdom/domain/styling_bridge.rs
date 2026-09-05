use super::identifier::{InteractionContext, NodePath};
use super::kind::VNodeKind;
use super::vnode::VNode;
use crate::features::layout_engine::domain::StyledNode;
use crate::features::styling::domain::{
    ClassNameList, ComputedStyle, ElementQuery, PseudoClass,
};
use crate::features::styling::ports::StyleResolverPort;
use crate::shared::primitives::ModuleKey;

impl VNode {
    #[must_use]
    pub fn resolve_styles(
        &self,
        resolver: &dyn StyleResolverPort,
        interaction: Option<&InteractionContext>,
        parent: Option<&ElementQuery>,
    ) -> StyledNode {
        self.resolve_styles_recursive(resolver, interaction, &NodePath::root(), 0, 1, parent)
    }

    fn resolve_styles_recursive(
        &self,
        resolver: &dyn StyleResolverPort,
        interaction: Option<&InteractionContext>,
        path: &NodePath,
        child_index: usize,
        total_children: usize,
        parent: Option<&ElementQuery>,
    ) -> StyledNode {
        let pseudo_classes = compute_pseudo_classes(path, interaction);
        let classes_slice = self.class_names().map_or(&[][..], ClassNameList::as_slice);
        let query = ElementQuery::new(
            self.tag().as_str(),
            self.element_id(),
            classes_slice,
            &pseudo_classes,
            parent,
        )
        .with_structural_context(child_index, total_children, self.is_empty());

        let mut style = match self.kind() {
            VNodeKind::Flex { .. } => ComputedStyle::default_for_flex(),
            VNodeKind::Grid { .. } => ComputedStyle::default_for_grid(),
            _ => ComputedStyle::default(),
        };
        style.merge_with(&resolver.resolve_style(&query));

        let styled_tooltip = self.tooltip().map(|t| {
            Box::new(t.resolve_styles_recursive(resolver, interaction, &path.child(0), 0, 1, None))
        });
        let styled_popup = self.popup().map(|p| {
            Box::new(p.resolve_styles_recursive(resolver, interaction, &path.child(0), 0, 1, None))
        });

        let styled_children = match self.kind() {
            VNodeKind::Flex { children } | VNodeKind::Grid { children } => {
                let total = children.len();
                children
                    .iter()
                    .enumerate()
                    .map(|(idx, child)| {
                        child.resolve_styles_recursive(
                            resolver,
                            interaction,
                            &path.child(idx),
                            idx,
                            total,
                            Some(&query),
                        )
                    })
                    .collect()
            }
            _ => Vec::new(),
        };

        self.map_to_styled_node(path, style, styled_tooltip, styled_popup, styled_children)
    }

    fn map_to_styled_node(
        &self,
        path: &NodePath,
        style: ComputedStyle,
        styled_tooltip: Option<Box<StyledNode>>,
        styled_popup: Option<Box<StyledNode>>,
        styled_children: Vec<StyledNode>,
    ) -> StyledNode {
        match self.kind() {
            VNodeKind::Flex { .. } => StyledNode::Flex {
                path: path.clone(),
                children: styled_children,
                style,
                on_click: self.on_click().cloned(),
                on_hover: self.on_hover().cloned(),
                tooltip: styled_tooltip,
                popup: styled_popup,
            },
            VNodeKind::Grid { .. } => StyledNode::Grid {
                path: path.clone(),
                children: styled_children,
                style,
                on_click: self.on_click().cloned(),
                on_hover: self.on_hover().cloned(),
                tooltip: styled_tooltip,
                popup: styled_popup,
            },
            VNodeKind::Text { text } => StyledNode::Text {
                path: path.clone(),
                text: text.clone(),
                style,
                on_click: self.on_click().cloned(),
                on_hover: self.on_hover().cloned(),
                tooltip: styled_tooltip,
                popup: styled_popup,
            },
            VNodeKind::Progress { value, orientation } => StyledNode::Progress {
                path: path.clone(),
                value: *value,
                orientation: *orientation,
                style,
                on_click: self.on_click().cloned(),
                on_hover: self.on_hover().cloned(),
                tooltip: styled_tooltip,
                popup: styled_popup,
            },
            VNodeKind::Rect => StyledNode::Rect {
                path: path.clone(),
                style,
                on_click: self.on_click().cloned(),
                on_hover: self.on_hover().cloned(),
                tooltip: styled_tooltip,
                popup: styled_popup,
            },
            VNodeKind::Image { data, pixel_size } => StyledNode::Image {
                path: path.clone(),
                data: data.clone(),
                pixel_size: *pixel_size,
                style,
                tooltip: styled_tooltip,
                popup: styled_popup,
            },
            VNodeKind::Module {
                name,
                instance_id,
                options,
            } => StyledNode::Module {
                path: path.clone(),
                key: ModuleKey::new(name.clone(), instance_id.clone()),
                options: options.clone(),
                style,
                on_click: self.on_click().cloned(),
                on_hover: self.on_hover().cloned(),
                tooltip: styled_tooltip,
                popup: styled_popup,
            },
        }
    }
}

fn compute_pseudo_classes(
    path: &NodePath,
    interaction: Option<&InteractionContext>,
) -> Vec<PseudoClass> {
    let mut pseudo_classes = Vec::new();
    let Some(ctx) = interaction else {
        return pseudo_classes;
    };

    if ctx
        .hovered_path()
        .is_some_and(|h| h == path || h.starts_with(path))
    {
        pseudo_classes.push(PseudoClass::Hover);
    }
    if ctx
        .active_path()
        .is_some_and(|a| a == path || a.starts_with(path))
    {
        pseudo_classes.push(PseudoClass::Active);
    }
    if ctx.focused_path().is_some_and(|f| f == path) {
        pseudo_classes.push(PseudoClass::Focused);
    }
    if path.is_root() && ctx.is_monitor_focused() && !pseudo_classes.contains(&PseudoClass::Focused)
    {
        pseudo_classes.push(PseudoClass::Focused);
    }

    pseudo_classes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::vdom::domain::TextContent;

    struct MockResolver;
    impl StyleResolverPort for MockResolver {
        fn resolve_style(
            &self,
            _query: &ElementQuery,
        ) -> crate::features::styling::domain::ComputedStyle {
            crate::features::styling::domain::ComputedStyle::default()
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
        .with_popup(Box::new(popup_content.clone()));

        assert!(button.popup().is_some());
        assert_eq!(button.popup().unwrap(), &popup_content);

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
        assert_eq!(grid_node.tag(), super::super::tag::NodeTag::Grid);
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
}
