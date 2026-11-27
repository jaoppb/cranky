mod floating;

use super::anchor::ResolvedInteraction;
use super::mapper::{StyledParts, map_to_styled_node};
use super::pseudo::compute_pseudo_classes;
use crate::features::layout_engine::domain::StyledNode;
use crate::features::styling::domain::{
    ClassNameList, ComputedStyle, ElementIdentity, ElementQuery, Emptiness, InheritedStyle,
    PseudoClass, SiblingPosition,
};
use crate::features::styling::ports::StyleResolverPort;
use crate::features::vdom::domain::{InteractionContext, NodePath, SurfaceSpace, VNode, VNodeKind};

/// One surface tree's styling pass: the resolver every node's CSS comes from, the raw
/// interaction context (so a floating child can re-resolve its own `ResolvedInteraction` against
/// its own tree below) and the interaction already resolved against *this* surface's tree (what
/// pseudo-class computation for its nodes uses). Each floating kind starts a pass of its own.
#[derive(Clone, Copy)]
struct SurfacePass<'a> {
    resolver: &'a dyn StyleResolverPort,
    ctx: Option<&'a InteractionContext>,
    resolved: Option<&'a ResolvedInteraction>,
}

impl<'a> SurfacePass<'a> {
    /// The same pass over another surface tree, whose interaction was resolved as `resolved`.
    const fn on_tree(self, resolved: Option<&'a ResolvedInteraction>) -> Self {
        Self { resolved, ..self }
    }
}

/// What a parent hands each child it resolves: the child's path, its position among its
/// siblings, the parent's own query (for descendant and child selectors) and the style the child
/// inherits. A surface root gets a fresh one.
struct ChildSite<'a> {
    path: &'a NodePath,
    position: SiblingPosition,
    parent: Option<&'a ElementQuery<'a>>,
    inherited: &'a InheritedStyle,
}

/// What a resolved node hands its children: its path, its own query (for descendant and child
/// selectors) and what they inherit — its own resolved style (color/font-size/custom properties),
/// after its own cascade (including inheritance from further up) was applied.
struct ParentSite<'a> {
    path: &'a NodePath,
    query: &'a ElementQuery<'a>,
    inherited: &'a InheritedStyle,
}

impl VNode {
    #[must_use]
    pub(crate) fn resolve_styles(
        &self,
        resolver: &dyn StyleResolverPort,
        interaction: Option<&InteractionContext>,
    ) -> StyledNode {
        let resolved_interaction =
            interaction.map(|ctx| ResolvedInteraction::resolve(self, SurfaceSpace::Bar, ctx));
        self.resolve_styles_recursive(
            SurfacePass {
                resolver,
                ctx: interaction,
                resolved: resolved_interaction.as_ref(),
            },
            &ChildSite {
                path: &NodePath::root_in(SurfaceSpace::Bar),
                position: SiblingPosition::root(),
                parent: None,
                inherited: &InheritedStyle::default(),
            },
        )
    }

    fn resolve_styles_recursive(&self, pass: SurfacePass<'_>, site: &ChildSite<'_>) -> StyledNode {
        let pseudo_classes = compute_pseudo_classes(site.path, pass.resolved);
        let query = self.element_query(site, &pseudo_classes);
        let (matched_style, child_inherited) = pass.resolver.resolve_style(&query, site.inherited);
        let mut style = self.kind_default_style();
        style.merge_with(&matched_style);
        let floating = self.resolve_floating(pass, &query);
        let owner = ParentSite {
            path: site.path,
            query: &query,
            inherited: &child_inherited,
        };
        let children = self.resolve_children(pass, &owner);
        map_to_styled_node(
            self,
            site.path,
            StyledParts {
                style,
                floating,
                children,
            },
        )
    }

    /// What this node's selectors are matched with at `site`, given its active pseudo-classes.
    fn element_query<'a>(
        &'a self,
        site: &ChildSite<'a>,
        pseudo_classes: &'a [PseudoClass],
    ) -> ElementQuery<'a> {
        let classes = self.class_names().map_or(&[][..], ClassNameList::as_slice);
        ElementQuery::new(
            ElementIdentity::new(self.tag().as_str(), self.element_id(), classes),
            pseudo_classes,
            site.parent,
        )
        .with_structural_context(site.position, Emptiness::from(self.is_empty()))
    }

    /// The style a node of this kind starts from before its matched rules are laid over it.
    fn kind_default_style(&self) -> ComputedStyle {
        match self.kind() {
            VNodeKind::Flex { .. } => ComputedStyle::default_for_flex(),
            VNodeKind::Grid { .. } => ComputedStyle::default_for_grid(),
            _ => ComputedStyle::default(),
        }
    }

    /// Resolves each child in order, at its index among its siblings below `owner`.
    fn resolve_children(&self, pass: SurfacePass<'_>, owner: &ParentSite<'_>) -> Vec<StyledNode> {
        let children = self.children();
        let total = children.len();
        children
            .iter()
            .enumerate()
            .map(|(index, child)| {
                child.resolve_styles_recursive(
                    pass,
                    &ChildSite {
                        path: &owner.path.child(index),
                        position: SiblingPosition::new(index, total),
                        parent: Some(owner.query),
                        inherited: owner.inherited,
                    },
                )
            })
            .collect()
    }
}
