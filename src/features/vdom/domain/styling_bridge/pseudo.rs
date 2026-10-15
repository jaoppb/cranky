use super::anchor::ResolvedInteraction;
use crate::features::styling::domain::PseudoClass;
use crate::features::vdom::domain::{NodePath, SurfaceSpace};

#[must_use]
pub fn compute_pseudo_classes(
    path: &NodePath,
    resolved: Option<&ResolvedInteraction>,
) -> Vec<PseudoClass> {
    let Some(resolved) = resolved else {
        return Vec::new();
    };

    let mut pseudo_classes = Vec::new();
    if resolved.is_hovered_or_ancestor(path) {
        pseudo_classes.push(PseudoClass::Hover);
    }
    if resolved.is_active_or_ancestor(path) {
        pseudo_classes.push(PseudoClass::Active);
    }
    // The monitor-focus fallback means "this is the bar's own root", not
    // "this is root of whatever surface it happens to be in" — a popup or
    // panel has its own root now too, and it must not pick up :focus just
    // because the underlying monitor does.
    if resolved.is_focused(path)
        || (path.is_root() && path.surface() == SurfaceSpace::Bar && resolved.is_monitor_focused())
    {
        pseudo_classes.push(PseudoClass::Focused);
    }

    pseudo_classes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::vdom::domain::{InteractionContext, NodeRef, VNode};

    /// `ResolvedInteraction::resolve` only needs to walk the tree when an
    /// anchor is non-empty; every test here captures a keyless ref, so an
    /// empty tree is a sufficient stand-in for "some surface's own root".
    fn empty_root() -> VNode {
        VNode::new_flex(Vec::new(), None, None, None, None, None)
    }

    #[test]
    fn test_hover_does_not_cross_surface_boundary() {
        // Owner node in the bar tree; its popup's content lives in a
        // separate surface. Same numeric indices on purpose — the surface
        // is what must keep them apart.
        let owner_path = NodePath::new(SurfaceSpace::Bar, vec![0, 1]);

        let hovered_in_popup =
            NodeRef::new(Vec::new(), NodePath::new(SurfaceSpace::Popup, vec![0, 1]));
        let interaction = InteractionContext::new(Some(hovered_in_popup), None, None, false);

        // Hovering inside the popup must not mark the bar node that owns it
        // as :hover, even though the indices happen to line up.
        let resolved_for_bar =
            ResolvedInteraction::resolve(&empty_root(), SurfaceSpace::Bar, &interaction);
        assert!(
            !compute_pseudo_classes(&owner_path, Some(&resolved_for_bar))
                .contains(&PseudoClass::Hover)
        );

        // The popup node actually under the cursor still gets :hover, and so
        // does an ancestor within the SAME surface.
        let resolved_for_popup =
            ResolvedInteraction::resolve(&empty_root(), SurfaceSpace::Popup, &interaction);
        let popup_node_path = NodePath::new(SurfaceSpace::Popup, vec![0, 1]);
        assert!(
            compute_pseudo_classes(&popup_node_path, Some(&resolved_for_popup))
                .contains(&PseudoClass::Hover)
        );

        let popup_ancestor_path = NodePath::new(SurfaceSpace::Popup, vec![0]);
        assert!(
            compute_pseudo_classes(&popup_ancestor_path, Some(&resolved_for_popup))
                .contains(&PseudoClass::Hover)
        );
    }

    #[test]
    fn test_monitor_focus_fallback_is_bar_only() {
        // is_root() is true for a popup's own root too now — the
        // monitor-focus fallback must not spuriously mark a popup/panel
        // root :focus just because the underlying monitor has focus.
        let interaction = InteractionContext::new(None, None, None, true);
        let resolved = ResolvedInteraction::resolve(&empty_root(), SurfaceSpace::Bar, &interaction);

        let bar_root = NodePath::root_in(SurfaceSpace::Bar);
        assert!(compute_pseudo_classes(&bar_root, Some(&resolved)).contains(&PseudoClass::Focused));

        let popup_root = NodePath::root_in(SurfaceSpace::Popup);
        assert!(
            !compute_pseudo_classes(&popup_root, Some(&resolved)).contains(&PseudoClass::Focused)
        );

        let panel_root = NodePath::root_in(SurfaceSpace::Panel);
        assert!(
            !compute_pseudo_classes(&panel_root, Some(&resolved)).contains(&PseudoClass::Focused)
        );
    }
}
