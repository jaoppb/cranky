//! Resolves the floating contents a node owns — tooltip, popup and panel — each as the root of its
//! own surface tree.
use super::{ChildSite, SurfacePass};
use crate::features::layout_engine::domain::{StyledNode, StyledPanel, StyledPopup};
use crate::features::styling::domain::{ElementQuery, InheritedStyle, SiblingPosition};
use crate::features::vdom::domain::styling_bridge::anchor::ResolvedInteraction;
use crate::features::vdom::domain::styling_bridge::mapper::StyledFloating;
use crate::features::vdom::domain::{NodePath, SurfaceSpace, VNode};

/// What a node hands the popup and panel contents it owns: its query, as their parent, and the
/// fresh style they inherit.
struct FloatingOwner<'a> {
    query: &'a ElementQuery<'a>,
    inherited: &'a InheritedStyle,
}

impl<'a> FloatingOwner<'a> {
    /// The root of `surface`'s own tree, under this owner.
    const fn root_of(&self, surface: SurfaceSpace) -> SurfaceRoot<'a> {
        SurfaceRoot {
            surface,
            parent: Some(self.query),
            inherited: self.inherited,
        }
    }
}

/// The root of a floating surface's own tree: which surface, the parent query its selectors see
/// (if any) and the style it inherits.
struct SurfaceRoot<'a> {
    surface: SurfaceSpace,
    parent: Option<&'a ElementQuery<'a>>,
    inherited: &'a InheritedStyle,
}

impl VNode {
    /// Resolves the tooltip, popup and panel contents. Each floating kind gets its own surface
    /// root rather than descending from this node's path — a popup, panel, or tooltip is a
    /// different tree from the one that owns it, not a subtree of it, so its nodes must not be
    /// ancestor-matched (`starts_with`) against the owner's. Each content also inherits fresh,
    /// rather than picking up the owning element's color/font.
    pub(super) fn resolve_floating(
        &self,
        pass: SurfacePass<'_>,
        query: &ElementQuery<'_>,
    ) -> StyledFloating {
        let detached_inherited = InheritedStyle::default();
        let owner = FloatingOwner {
            query,
            inherited: &detached_inherited,
        };
        StyledFloating {
            tooltip: self.resolve_tooltip(pass, &detached_inherited),
            popup: self.resolve_popup(pass, &owner),
            panel: self.resolve_panel(pass, &owner),
        }
    }

    /// The tooltip content, a root for selector matching with no parent query.
    fn resolve_tooltip(
        &self,
        pass: SurfacePass<'_>,
        inherited: &InheritedStyle,
    ) -> Option<Box<StyledNode>> {
        self.tooltip().map(|tooltip_node| {
            let root = SurfaceRoot {
                surface: SurfaceSpace::Tooltip,
                parent: None,
                inherited,
            };
            Box::new(tooltip_node.resolve_as_surface_root(pass, &root))
        })
    }

    /// The popup content, matched with the owner as its parent, and the popup's placement.
    fn resolve_popup(
        &self,
        pass: SurfacePass<'_>,
        owner: &FloatingOwner<'_>,
    ) -> Option<StyledPopup> {
        self.popup().map(|popup_spec| {
            let root = owner.root_of(SurfaceSpace::Popup);
            let content = Box::new(popup_spec.content().resolve_as_surface_root(pass, &root));
            StyledPopup::new(content, popup_spec.anchor_direction(), popup_spec.offset())
        })
    }

    /// The panel content, matched with the owner as its parent, and the panel's surface.
    fn resolve_panel(
        &self,
        pass: SurfacePass<'_>,
        owner: &FloatingOwner<'_>,
    ) -> Option<StyledPanel> {
        self.panel().map(|panel_spec| {
            let root = owner.root_of(SurfaceSpace::Panel);
            let content = Box::new(panel_spec.content().resolve_as_surface_root(pass, &root));
            StyledPanel::new(content, panel_spec.surface())
        })
    }

    /// Resolves this node as the root of `root.surface`'s own tree, against the interaction
    /// resolved for that tree.
    fn resolve_as_surface_root(&self, pass: SurfacePass<'_>, root: &SurfaceRoot<'_>) -> StyledNode {
        let resolved = pass
            .ctx
            .map(|ctx| ResolvedInteraction::resolve(self, root.surface, ctx));
        self.resolve_styles_recursive(
            pass.on_tree(resolved.as_ref()),
            &ChildSite {
                path: &NodePath::root_in(root.surface),
                position: SiblingPosition::root(),
                parent: root.parent,
                inherited: root.inherited,
            },
        )
    }
}
