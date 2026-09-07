use super::mapper::map_to_styled_node;
use super::pseudo::compute_pseudo_classes;
use crate::features::layout_engine::domain::{StyledNode, StyledPanel, StyledPopup};
use crate::features::styling::domain::{ClassNameList, ComputedStyle, ElementQuery};
use crate::features::styling::ports::StyleResolverPort;
use crate::features::vdom::domain::{InteractionContext, NodePath, VNode, VNodeKind};

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
        let classes = self.class_names().map_or(&[][..], ClassNameList::as_slice);
        let query = ElementQuery::new(
            self.tag().as_str(),
            self.element_id(),
            classes,
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

        let tooltip = self.tooltip().map(|t| {
            Box::new(t.resolve_styles_recursive(resolver, interaction, &path.child(0), 0, 1, None))
        });
        let popup = self.popup().map(|p| {
            let content = Box::new(p.content().resolve_styles_recursive(
                resolver,
                interaction,
                &NodePath::root(),
                0,
                1,
                Some(&query),
            ));
            StyledPopup::new(
                content,
                p.anchor_direction(),
                p.offset(),
                p.dismiss_on_unfocus(),
            )
        });
        let panel = self.panel().map(|p| {
            let content = Box::new(p.content().resolve_styles_recursive(
                resolver,
                interaction,
                &NodePath::root(),
                0,
                1,
                Some(&query),
            ));
            StyledPanel::new(
                content,
                p.layer(),
                p.anchor(),
                *p.margin(),
                p.exclusive_zone(),
                p.keyboard(),
            )
        });

        let children = match self.kind() {
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

        map_to_styled_node(self, path, style, tooltip, popup, panel, children)
    }
}
