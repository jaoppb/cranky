//! Resolves the floating contents a node owns — tooltip, popup and panel — each as the root of its
//! own surface tree.
use super::{ChildSite, SurfacePass};
use crate::features::layout_engine::domain::{StyledNode, StyledPanel, StyledPopup};
use crate::features::styling::domain::{ElementQuery, InheritedStyle, SiblingPosition};
use crate::features::vdom::domain::styling_bridge::anchor::ResolvedInteraction;
use crate::features::vdom::domain::styling_bridge::mapper::StyledFloating;
use crate::features::vdom::domain::{NodePath, SurfaceSpace, VNode};

/// The root of a floating surface's own tree: which surface, and the parent query its
/// selectors see (if any). A root inherits nothing: its style starts fresh.
struct SurfaceRoot<'a> {
    surface: SurfaceSpace,
    parent: Option<&'a ElementQuery<'a>>,
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
        StyledFloating {
            tooltip: self.resolve_tooltip(pass),
            popup: self.resolve_popup(pass, query),
            panel: self.resolve_panel(pass, query),
        }
    }

    /// The tooltip content, a root for selector matching with no parent query.
    fn resolve_tooltip(&self, pass: SurfacePass<'_>) -> Option<Box<StyledNode>> {
        self.tooltip().map(|tooltip_node| {
            let root = SurfaceRoot {
                surface: SurfaceSpace::Tooltip,
                parent: None,
            };
            Box::new(tooltip_node.resolve_as_surface_root(pass, &root))
        })
    }

    /// The popup content, matched with its owner's `query` as its parent, and the popup's
    /// placement.
    fn resolve_popup(
        &self,
        pass: SurfacePass<'_>,
        query: &ElementQuery<'_>,
    ) -> Option<StyledPopup> {
        self.popup().map(|popup_spec| {
            let root = SurfaceRoot {
                surface: SurfaceSpace::Popup,
                parent: Some(query),
            };
            let content = Box::new(popup_spec.content().resolve_as_surface_root(pass, &root));
            StyledPopup::new(content, popup_spec.anchor_direction(), popup_spec.offset())
        })
    }

    /// The panel content, matched with its owner's `query` as its parent, and the panel's
    /// surface.
    fn resolve_panel(
        &self,
        pass: SurfacePass<'_>,
        query: &ElementQuery<'_>,
    ) -> Option<StyledPanel> {
        self.panel().map(|panel_spec| {
            let root = SurfaceRoot {
                surface: SurfaceSpace::Panel,
                parent: Some(query),
            };
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
                inherited: &InheritedStyle::default(),
            },
        )
    }
}
