use super::anchor::ResolvedInteraction;
use super::mapper::{StyledParts, map_to_styled_node};
use super::pseudo::compute_pseudo_classes;
use crate::features::layout_engine::domain::{StyledNode, StyledPanel, StyledPopup};
use crate::features::styling::domain::{
    ClassNameList, ComputedStyle, ElementIdentity, ElementQuery, Emptiness, InheritedStyle,
    SiblingPosition,
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
        let classes = self.class_names().map_or(&[][..], ClassNameList::as_slice);
        let query = ElementQuery::new(
            ElementIdentity::new(self.tag().as_str(), self.element_id(), classes),
            &pseudo_classes,
            site.parent,
        )
        .with_structural_context(site.position, Emptiness::from(self.is_empty()));

        let mut style = match self.kind() {
            VNodeKind::Flex { .. } => ComputedStyle::default_for_flex(),
            VNodeKind::Grid { .. } => ComputedStyle::default_for_grid(),
            _ => ComputedStyle::default(),
        };
        // `child_inherited` is what this node's own children inherit from —
        // its own resolved style (color/font-size/custom properties),
        // after its own cascade (including inheritance from further up)
        // was applied.
        let (matched_style, child_inherited) = pass.resolver.resolve_style(&query, site.inherited);
        style.merge_with(&matched_style);
        // A tooltip/popup/panel's content is its own root for selector
        // matching (`parent: None`, below) — it inherits fresh for the same
        // reason, rather than picking up the owning element's color/font.
        let detached_inherited = InheritedStyle::default();

        // Each floating kind gets its own surface root rather than
        // descending from `path` — a popup, panel, or tooltip is a different
        // tree from the one that owns it, not a subtree of it, so its nodes
        // must not be ancestor-matched (`starts_with`) against the owner's.
        let tooltip = self.tooltip().map(|tooltip_node| {
            let tooltip_resolved = pass
                .ctx
                .map(|ctx| ResolvedInteraction::resolve(tooltip_node, SurfaceSpace::Tooltip, ctx));
            Box::new(tooltip_node.resolve_styles_recursive(
                pass.on_tree(tooltip_resolved.as_ref()),
                &ChildSite {
                    path: &NodePath::root_in(SurfaceSpace::Tooltip),
                    position: SiblingPosition::root(),
                    parent: None,
                    inherited: &detached_inherited,
                },
            ))
        });
        let popup = self.popup().map(|popup_spec| {
            let popup_resolved = pass.ctx.map(|ctx| {
                ResolvedInteraction::resolve(popup_spec.content(), SurfaceSpace::Popup, ctx)
            });
            let content = Box::new(popup_spec.content().resolve_styles_recursive(
                pass.on_tree(popup_resolved.as_ref()),
                &ChildSite {
                    path: &NodePath::root_in(SurfaceSpace::Popup),
                    position: SiblingPosition::root(),
                    parent: Some(&query),
                    inherited: &detached_inherited,
                },
            ));
            StyledPopup::new(content, popup_spec.anchor_direction(), popup_spec.offset())
        });
        let panel = self.panel().map(|panel_spec| {
            let panel_resolved = pass.ctx.map(|ctx| {
                ResolvedInteraction::resolve(panel_spec.content(), SurfaceSpace::Panel, ctx)
            });
            let content = Box::new(panel_spec.content().resolve_styles_recursive(
                pass.on_tree(panel_resolved.as_ref()),
                &ChildSite {
                    path: &NodePath::root_in(SurfaceSpace::Panel),
                    position: SiblingPosition::root(),
                    parent: Some(&query),
                    inherited: &detached_inherited,
                },
            ));
            StyledPanel::new(content, panel_spec.surface())
        });

        let children = match self.kind() {
            VNodeKind::Flex { children } | VNodeKind::Grid { children } => {
                let total = children.len();
                children
                    .iter()
                    .enumerate()
                    .map(|(idx, child)| {
                        child.resolve_styles_recursive(
                            pass,
                            &ChildSite {
                                path: &site.path.child(idx),
                                position: SiblingPosition::new(idx, total),
                                parent: Some(&query),
                                inherited: &child_inherited,
                            },
                        )
                    })
                    .collect()
            }
            _ => Vec::new(),
        };

        map_to_styled_node(
            self,
            site.path,
            StyledParts {
                style,
                tooltip,
                popup,
                panel,
                children,
            },
        )
    }
}
