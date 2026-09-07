use crate::features::styling::domain::PseudoClass;
use crate::features::vdom::domain::{InteractionContext, NodePath};

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
    if ctx.focused_path().is_some_and(|f| f == path)
        || (path.is_root() && ctx.is_monitor_focused())
    {
        pseudo_classes.push(PseudoClass::Focused);
    }

    pseudo_classes
}
