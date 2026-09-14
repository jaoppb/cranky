use super::mapper::map_to_styled_node;
use super::pseudo::compute_pseudo_classes;
use crate::features::layout_engine::domain::{StyledNode, StyledPanel, StyledPopup};
use crate::features::styling::domain::{
    ClassNameList, ComputedStyle, ElementQuery, InheritedStyle,
};
use crate::features::styling::ports::StyleResolverPort;
use crate::features::vdom::domain::{InteractionContext, NodePath, VNode, VNodeKind};

/// Where a node sits among its siblings — bundled so the recursive walk
/// stays within clippy's argument-count limit.
#[derive(Debug, Clone, Copy)]
struct SiblingPosition {
    index: usize,
    total: usize,
}

impl SiblingPosition {
    const fn root() -> Self {
        Self { index: 0, total: 1 }
    }

    const fn new(index: usize, total: usize) -> Self {
        Self { index, total }
    }

    const fn index(self) -> usize {
        self.index
    }

    const fn total(self) -> usize {
        self.total
    }
}

impl VNode {
    #[must_use]
    pub fn resolve_styles(
        &self,
        resolver: &dyn StyleResolverPort,
        interaction: Option<&InteractionContext>,
        parent: Option<&ElementQuery>,
    ) -> StyledNode {
        self.resolve_styles_recursive(
            resolver,
            interaction,
            &NodePath::root(),
            SiblingPosition::root(),
            parent,
            &InheritedStyle::default(),
        )
    }

    fn resolve_styles_recursive(
        &self,
        resolver: &dyn StyleResolverPort,
        interaction: Option<&InteractionContext>,
        path: &NodePath,
        position: SiblingPosition,
        parent: Option<&ElementQuery>,
        inherited: &InheritedStyle,
    ) -> StyledNode {
        let pseudo_classes = compute_pseudo_classes(path, self.key(), interaction);
        let classes = self.class_names().map_or(&[][..], ClassNameList::as_slice);
        let query = ElementQuery::new(
            self.tag().as_str(),
            self.element_id(),
            classes,
            &pseudo_classes,
            parent,
        )
        .with_structural_context(position.index(), position.total(), self.is_empty());

        let mut style = match self.kind() {
            VNodeKind::Flex { .. } => ComputedStyle::default_for_flex(),
            VNodeKind::Grid { .. } => ComputedStyle::default_for_grid(),
            _ => ComputedStyle::default(),
        };
        // `child_inherited` is what this node's own children inherit from —
        // its own resolved style (color/font-size/custom properties),
        // after its own cascade (including inheritance from further up)
        // was applied.
        let (matched_style, child_inherited) = resolver.resolve_style(&query, inherited);
        style.merge_with(&matched_style);
        // A tooltip/popup/panel's content is its own root for selector
        // matching (`parent: None`, below) — it inherits fresh for the same
        // reason, rather than picking up the owning element's color/font.
        let detached_inherited = InheritedStyle::default();

        let tooltip = self.tooltip().map(|t| {
            Box::new(t.resolve_styles_recursive(
                resolver,
                interaction,
                &path.child(0),
                SiblingPosition::root(),
                None,
                &detached_inherited,
            ))
        });
        let popup = self.popup().map(|p| {
            let content = Box::new(p.content().resolve_styles_recursive(
                resolver,
                interaction,
                &NodePath::root(),
                SiblingPosition::root(),
                Some(&query),
                &detached_inherited,
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
                SiblingPosition::root(),
                Some(&query),
                &detached_inherited,
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
                            SiblingPosition::new(idx, total),
                            Some(&query),
                            &child_inherited,
                        )
                    })
                    .collect()
            }
            _ => Vec::new(),
        };

        map_to_styled_node(self, path, style, tooltip, popup, panel, children)
    }
}
