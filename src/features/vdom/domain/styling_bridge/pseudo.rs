use super::anchor::ResolvedInteraction;
use crate::features::styling::domain::PseudoClass;
use crate::features::vdom::domain::NodePath;

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
    if resolved.is_focused(path) || (path.is_root() && resolved.is_monitor_focused()) {
        pseudo_classes.push(PseudoClass::Focused);
    }

    pseudo_classes
}
