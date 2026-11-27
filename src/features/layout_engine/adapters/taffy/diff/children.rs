use super::super::reconciler::{ChildrenPatch, LayoutState, Patch, Update};
use super::super::style_builder::node_to_style;
use crate::features::layout_engine::domain::{StyledNode, TextMeasurer};
use crate::features::vdom::domain::NodeKey;
use std::collections::{HashMap, HashSet};
use taffy::style::Style;
use taffy::tree::NodeId;

/// Whether every key in `keys` is present and unique. Vacuously true for an
/// empty iterator, so callers must also check that at least one child was
/// actually keyed before trusting this as "this list may use keyed
/// reconciliation".
fn fully_keyed<'a>(keys: impl Iterator<Item = Option<&'a str>>) -> bool {
    let mut seen = HashSet::new();
    for key in keys {
        match key {
            Some(name) if seen.insert(name) => {}
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
    for (index, new_child) in new_children.iter().enumerate() {
        if let Some(old_child) = old_children.get(index) {
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
        .map(|old_child| old_child.root_node)
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
    let old_by_key = index_by_key(old_children);
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
    let removed = unmatched_nodes(old_children, &matched_keys);
    ChildrenPatch { patches, removed }
}

/// The keyed children of `old_children`, by key.
fn index_by_key(old_children: &[LayoutState]) -> HashMap<&str, &LayoutState> {
    let mut old_by_key = HashMap::with_capacity(old_children.len());
    for old_child in old_children {
        if let Some(key) = old_child.layout.node_key() {
            old_by_key.insert(key.as_str(), old_child);
        }
    }
    old_by_key
}

/// The nodes of the `old_children` whose key no new child matched, or that had no key.
fn unmatched_nodes(old_children: &[LayoutState], matched_keys: &HashSet<&str>) -> Vec<NodeId> {
    old_children
        .iter()
        .filter(|old_child| {
            old_child
                .layout
                .node_key()
                .is_none_or(|key| !matched_keys.contains(key.as_str()))
        })
        .map(|old_child| old_child.root_node)
        .collect()
}

pub(super) fn diff_container<'a>(
    old_state: &'a LayoutState,
    new_layout: &'a StyledNode,
    measurer: &mut dyn TextMeasurer,
) -> Patch<'a> {
    let new_children = new_layout.children();
    let style = changed_style(old_state, new_layout, measurer);
    let old_children = old_state.children.as_slice();
    let keyed = reconciles_by_key(old_children, new_layout);
    let children = if keyed {
        diff_children_keyed(old_children, new_children, measurer)
    } else {
        diff_children_positional(old_children, new_children, measurer)
    };

    Patch::Update(Update {
        old_state,
        new_layout,
        style: Box::new(style),
        children: Some(children),
    })
}

/// The new taffy style of `new_layout`, or `None` when its computed style did not change.
fn changed_style(
    old_state: &LayoutState,
    new_layout: &StyledNode,
    measurer: &mut dyn TextMeasurer,
) -> Option<Style> {
    if old_state.layout.style() == new_layout.style() {
        None
    } else {
        Some(node_to_style(new_layout, measurer))
    }
}

/// Whether `new_layout`'s children are matched to `old_children` by key: when some child on
/// either side is keyed and every child on both sides has a unique key. Some keys but not that
/// is logged, and falls back to matching by position.
fn reconciles_by_key(old_children: &[LayoutState], new_layout: &StyledNode) -> bool {
    let new_children = new_layout.children();
    let old_keys = || {
        old_children
            .iter()
            .map(|old_child| old_child.layout.node_key().map(NodeKey::as_str))
    };
    let new_keys = || {
        new_children
            .iter()
            .map(|new_child| new_child.node_key().map(NodeKey::as_str))
    };
    let any_keyed = old_keys().any(|key| key.is_some()) || new_keys().any(|key| key.is_some());
    let keyed = any_keyed && fully_keyed(old_keys()) && fully_keyed(new_keys());
    if any_keyed && !keyed {
        log_mixed_keys(new_layout);
    }
    keyed
}

/// Logs that `new_layout`'s children mix keyed and unkeyed ones, or repeat a key.
fn log_mixed_keys(new_layout: &StyledNode) {
    tracing::warn!(
        path = ?new_layout.path(),
        "container has a mix of keyed and unkeyed children, or a duplicate key; \
         falling back to positional layout reconciliation for this render"
    );
}
