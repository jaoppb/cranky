use crate::features::styling::domain::PseudoClass;
use crate::features::vdom::domain::{InteractionContext, NodePath, SurfaceSpace};

#[must_use]
pub fn compute_pseudo_classes(
    path: &NodePath,
    interaction: Option<&InteractionContext>,
) -> Vec<PseudoClass> {
    let Some(ctx) = interaction else {
        return Vec::new();
    };

    let mut pseudo_classes = Vec::new();
    if ctx.hovered_path().is_some_and(|h| h == path || h.starts_with(path)) {
        pseudo_classes.push(PseudoClass::Hover);
    }
    if ctx.active_path().is_some_and(|a| a == path || a.starts_with(path)) {
        pseudo_classes.push(PseudoClass::Active);
    }
    // The monitor-focus fallback means "this is the bar's own root", not
    // "this is root of whatever surface it happens to be in" — a popup or
    // panel has its own root now too, and it must not pick up :focus just
    // because the underlying monitor does.
    if ctx.focused_path().is_some_and(|f| f == path)
        || (path.is_root() && path.surface() == SurfaceSpace::Bar && ctx.is_monitor_focused())
    {
        pseudo_classes.push(PseudoClass::Focused);
    }

    pseudo_classes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::vdom::domain::SurfaceSpace;

    #[test]
    fn test_hover_does_not_cross_surface_boundary() {
        // Owner node in the bar tree; its popup's content lives in a
        // separate surface. Same numeric indices on purpose — the surface
        // is what must keep them apart.
        let owner_path = NodePath::new(SurfaceSpace::Bar, vec![0, 1]);
        let popup_owner_path = NodePath::new(SurfaceSpace::Bar, vec![0, 1]);
        assert_eq!(owner_path, popup_owner_path);

        let hovered_in_popup = NodePath::new(SurfaceSpace::Popup, vec![0, 1]);
        let interaction = InteractionContext::new(Some(hovered_in_popup), None, None, false);

        // Hovering inside the popup must not mark the bar node that owns it
        // as :hover, even though the indices happen to line up.
        let owner_classes = compute_pseudo_classes(&owner_path, Some(&interaction));
        assert!(!owner_classes.contains(&PseudoClass::Hover));

        // The popup node actually under the cursor still gets :hover.
        let popup_node_path = NodePath::new(SurfaceSpace::Popup, vec![0, 1]);
        let popup_classes = compute_pseudo_classes(&popup_node_path, Some(&interaction));
        assert!(popup_classes.contains(&PseudoClass::Hover));

        // And an ancestor within the SAME surface still gets :hover, as before.
        let popup_ancestor_path = NodePath::new(SurfaceSpace::Popup, vec![0]);
        let ancestor_classes = compute_pseudo_classes(&popup_ancestor_path, Some(&interaction));
        assert!(ancestor_classes.contains(&PseudoClass::Hover));
    }

    #[test]
    fn test_monitor_focus_fallback_is_bar_only() {
        // is_root() is true for a popup's own root too now — the
        // monitor-focus fallback must not spuriously mark a popup/panel
        // root :focus just because the underlying monitor has focus.
        let interaction = InteractionContext::new(None, None, None, true);

        let bar_root = NodePath::root_in(SurfaceSpace::Bar);
        assert!(compute_pseudo_classes(&bar_root, Some(&interaction)).contains(&PseudoClass::Focused));

        let popup_root = NodePath::root_in(SurfaceSpace::Popup);
        assert!(!compute_pseudo_classes(&popup_root, Some(&interaction)).contains(&PseudoClass::Focused));

        let panel_root = NodePath::root_in(SurfaceSpace::Panel);
        assert!(!compute_pseudo_classes(&panel_root, Some(&interaction)).contains(&PseudoClass::Focused));
    }
}
