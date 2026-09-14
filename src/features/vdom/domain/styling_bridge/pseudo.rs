use crate::features::styling::domain::PseudoClass;
use crate::features::vdom::domain::{InteractionContext, NodeKey, NodePath, NodeRef};

/// Whether the node at `path` (with `key`, if it has one) is the node
/// `node_ref` was captured for. Matched by key first — robust across a
/// reorder or an earlier sibling's removal — falling back to `node_ref`'s
/// captured path when either side has no key. `allow_ancestor` additionally
/// treats `path` as a match when it's a prefix of the captured path, so a
/// pseudo-class can bubble from a hit leaf up to its containers; focus never
/// bubbles, so callers pass `false` for that check.
fn matches_ref(node_ref: &NodeRef, path: &NodePath, key: Option<&NodeKey>, allow_ancestor: bool) -> bool {
    if let (Some(a), Some(b)) = (node_ref.key(), key) {
        return a == b;
    }
    if allow_ancestor {
        path == node_ref.path() || path.starts_with(node_ref.path())
    } else {
        path == node_ref.path()
    }
}

#[must_use]
pub fn compute_pseudo_classes(
    path: &NodePath,
    key: Option<&NodeKey>,
    interaction: Option<&InteractionContext>,
) -> Vec<PseudoClass> {
    let Some(ctx) = interaction else {
        return Vec::new();
    };

    let mut pseudo_classes = Vec::new();
    if ctx.hovered().is_some_and(|h| matches_ref(h, path, key, true)) {
        pseudo_classes.push(PseudoClass::Hover);
    }
    if ctx.active().is_some_and(|a| matches_ref(a, path, key, true)) {
        pseudo_classes.push(PseudoClass::Active);
    }
    if ctx.focused().is_some_and(|f| matches_ref(f, path, key, false))
        || (path.is_root() && ctx.is_monitor_focused())
    {
        pseudo_classes.push(PseudoClass::Focused);
    }

    pseudo_classes
}
