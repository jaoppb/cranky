//! Keyed child reconciliation: matches the new children to the old ones by key.
use super::{ChildLists, insert_op, push_update, remove_op};
use crate::features::vdom::domain::{ChildPatchOp, Patch, VNode};
use std::collections::{HashMap, HashSet};

/// Each old child of a fully keyed list by its key, with its index.
fn index_by_key(children: &[VNode]) -> HashMap<&str, (usize, &VNode)> {
    let mut old_map = HashMap::new();
    for (idx, child) in children.iter().enumerate() {
        if let Some(key) = child.key() {
            old_map.insert(key.as_str(), (idx, child));
        }
    }
    old_map
}

/// A new child matched by key to an old one, with the index each sits at.
struct KeyedMatch<'a> {
    source: usize,
    target: usize,
    old_child: &'a VNode,
    new_child: &'a VNode,
}

/// Matches the new children to the old ones by key, in new-list order: a moved match is moved
/// then updated, an unmatched new child inserted; old children left unmatched are removed last,
/// in old-list order.
pub(super) fn keyed_patches<F>(lists: ChildLists<'_>, diff_nodes_fn: &F) -> Vec<ChildPatchOp>
where
    F: Fn(&VNode, &VNode) -> Patch,
{
    let old_map = index_by_key(lists.old);
    let mut matched_old_indices = HashSet::new();
    let mut child_patches = Vec::new();
    for (new_idx, new_child) in lists.new.iter().enumerate() {
        match new_child.key().and_then(|key| old_map.get(key.as_str())) {
            Some(&(old_idx, old_child)) => {
                matched_old_indices.insert(old_idx);
                let matched = KeyedMatch {
                    source: old_idx,
                    target: new_idx,
                    old_child,
                    new_child,
                };
                push_keyed_match(&mut child_patches, &matched, diff_nodes_fn);
            }
            None => child_patches.push(insert_op(new_idx, new_child)),
        }
    }
    push_unmatched_removals(&mut child_patches, lists.old, &matched_old_indices);
    child_patches
}

/// Pushes a `Move` when the match changed position, then its `Update` when its content changed.
fn push_keyed_match<F>(
    child_patches: &mut Vec<ChildPatchOp>,
    matched: &KeyedMatch<'_>,
    diff_nodes_fn: &F,
) where
    F: Fn(&VNode, &VNode) -> Patch,
{
    if matched.source != matched.target {
        child_patches.push(ChildPatchOp::Move {
            node_id: matched.old_child.node_id(),
            source: matched.source,
            target: matched.target,
        });
    }
    let patch = diff_nodes_fn(matched.old_child, matched.new_child);
    push_update(child_patches, matched.old_child, patch);
}

/// Pushes a `Remove` for every old child no new child matched, in old-list order.
fn push_unmatched_removals(
    child_patches: &mut Vec<ChildPatchOp>,
    old: &[VNode],
    matched_old_indices: &HashSet<usize>,
) {
    for (idx, child) in old.iter().enumerate() {
        if !matched_old_indices.contains(&idx) {
            child_patches.push(remove_op(child, idx));
        }
    }
}
