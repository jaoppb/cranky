use super::super::reconciler::{ChildrenPatch, LayoutState, Patch};
use super::super::style_builder::node_to_style;
use crate::features::layout_engine::domain::{StyledNode, TextMeasurer};
use crate::features::styling::domain::ComputedStyle;
use crate::features::vdom::domain::NodeKey;
use std::collections::{HashMap, HashSet};

/// Whether every key in `keys` is present and unique. Vacuously true for an
/// empty iterator, so callers must also check that at least one child was
/// actually keyed before trusting this as "this list may use keyed
/// reconciliation".
fn fully_keyed<'a>(keys: impl Iterator<Item = Option<&'a str>>) -> bool {
    let mut seen = HashSet::new();
    for key in keys {
        match key {
            Some(k) if seen.insert(k) => {}
            _ => return false,
        }
    }
    true
}

/// Matches `new_children` to `old_children` strictly by index — the
/// long-standing behavior, kept for containers that don't use `key`.
fn diff_children_positional<'a>(
    old_children: &'a [LayoutState],
    new_children: &'a [StyledNode],
    measurer: &mut dyn TextMeasurer,
) -> ChildrenPatch<'a> {
    let mut patches = Vec::with_capacity(new_children.len());
    for (i, new_child) in new_children.iter().enumerate() {
        if let Some(old_child) = old_children.get(i) {
            patches.push(super::diff(old_child, new_child, measurer));
        } else {
            patches.push(Patch::Create {
                new_layout: new_child,
            });
        }
    }

    let removed = old_children
        .iter()
        .skip(new_children.len())
        .map(|c| c.root_node)
        .collect();

    ChildrenPatch { patches, removed }
}

/// Matches `new_children` to `old_children` by `NodeKey`, so a container can
/// reorder, insert, or remove a middle child without every sibling after it
/// being torn down and rebuilt (the positional path's failure mode).
///
/// Requires every child on both sides to carry a unique key — enforced by
/// the caller via `fully_keyed` before this is ever reached.
fn diff_children_keyed<'a>(
    old_children: &'a [LayoutState],
    new_children: &'a [StyledNode],
    measurer: &mut dyn TextMeasurer,
) -> ChildrenPatch<'a> {
    let mut old_by_key: HashMap<&str, &'a LayoutState> = HashMap::with_capacity(old_children.len());
    for old_child in old_children {
        if let Some(key) = old_child.layout.node_key() {
            old_by_key.insert(key.as_str(), old_child);
        }
    }

    let mut matched_keys: HashSet<&str> = HashSet::with_capacity(new_children.len());
    let mut patches = Vec::with_capacity(new_children.len());
    for new_child in new_children {
        // `fully_keyed` gated entry into this function, so every new child
        // has a key.
        let Some(key) = new_child.node_key().map(NodeKey::as_str) else {
            patches.push(Patch::Create {
                new_layout: new_child,
            });
            continue;
        };
        if let Some(&old_child) = old_by_key.get(key) {
            matched_keys.insert(key);
            patches.push(super::diff(old_child, new_child, measurer));
        } else {
            patches.push(Patch::Create {
                new_layout: new_child,
            });
        }
    }

    let removed = old_children
        .iter()
        .filter(|c| {
            c.layout
                .node_key()
                .is_none_or(|key| !matched_keys.contains(key.as_str()))
        })
        .map(|c| c.root_node)
        .collect();

    ChildrenPatch { patches, removed }
}

pub(super) fn diff_container<'a>(
    old_state: &'a LayoutState,
    new_layout: &'a StyledNode,
    old_style: &ComputedStyle,
    new_style: &ComputedStyle,
    new_children: &'a [StyledNode],
    measurer: &mut dyn TextMeasurer,
) -> Patch<'a> {
    let style = if old_style == new_style {
        None
    } else {
        Some(node_to_style(new_layout, measurer))
    };

    let old_children = old_state.children.as_slice();
    let any_keyed = old_children
        .iter()
        .any(|c| c.layout.node_key().is_some())
        || new_children.iter().any(|c| c.node_key().is_some());
    let keyed = any_keyed
        && fully_keyed(old_children.iter().map(|c| c.layout.node_key().map(NodeKey::as_str)))
        && fully_keyed(new_children.iter().map(|c| c.node_key().map(NodeKey::as_str)));

    if any_keyed && !keyed {
        tracing::warn!(
            path = ?new_layout.path(),
            "container has a mix of keyed and unkeyed children, or a duplicate key; \
             falling back to positional layout reconciliation for this render"
        );
    }

    let children = if keyed {
        diff_children_keyed(old_children, new_children, measurer)
    } else {
        diff_children_positional(old_children, new_children, measurer)
    };

    Patch::Update {
        old_state,
        new_layout,
        style: Box::new(style),
        children: Some(children),
    }
}
