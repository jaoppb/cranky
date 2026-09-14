use crate::features::vdom::domain::{ChildPatchOp, NodeId, Patch, VNode};
use std::collections::{HashMap, HashSet};

/// Whether every child in `children` carries a key, with no duplicates.
/// Vacuously true for an empty slice — callers must separately check that at
/// least one child is keyed before treating that as "safe to key this list".
fn fully_keyed(children: &[VNode]) -> bool {
    let mut seen = HashSet::new();
    children
        .iter()
        .all(|c| c.key().is_some_and(|k| seen.insert(k.as_str())))
}

#[must_use]
pub fn diff_children_with<F>(
    parent_id: NodeId,
    old_children: &[VNode],
    new_children: &[VNode],
    diff_nodes_fn: F,
) -> Patch
where
    F: Fn(&VNode, &VNode) -> Patch,
{
    // Keyed reconciliation requires *every* sibling on both sides to carry a
    // unique key: a mix of keyed and unkeyed children, or a duplicate key,
    // has no well-defined match and silently produced wrong Move/Remove
    // pairs before — falling back to positional (with a warning) is honest
    // about not knowing which child is which.
    let any_keyed = old_children.iter().any(|c| c.key().is_some())
        || new_children.iter().any(|c| c.key().is_some());
    let keyed = any_keyed && fully_keyed(old_children) && fully_keyed(new_children);

    if any_keyed && !keyed {
        tracing::warn!(
            parent_id = %parent_id,
            "child list has a mix of keyed and unkeyed children, or a duplicate key; \
             falling back to positional reconciliation for this render"
        );
    }

    let mut child_patches = Vec::new();

    if keyed {
        let mut old_map: HashMap<&str, (usize, &VNode)> = HashMap::new();
        for (idx, child) in old_children.iter().enumerate() {
            if let Some(key) = child.key() {
                old_map.insert(key.as_str(), (idx, child));
            }
        }

        let mut matched_old_indices = std::collections::HashSet::new();

        for (new_idx, new_child) in new_children.iter().enumerate() {
            if let Some(key) = new_child.key() {
                if let Some(&(old_idx, old_child)) = old_map.get(key.as_str()) {
                    matched_old_indices.insert(old_idx);
                    if old_idx != new_idx {
                        child_patches.push(ChildPatchOp::Move {
                            node_id: old_child.node_id(),
                            from: old_idx,
                            to: new_idx,
                        });
                    }
                    let p = diff_nodes_fn(old_child, new_child);
                    if !p.is_no_change() {
                        child_patches.push(ChildPatchOp::Update {
                            node_id: old_child.node_id(),
                            patch: Box::new(p),
                        });
                    }
                } else {
                    child_patches.push(ChildPatchOp::Insert {
                        index: new_idx,
                        node: Box::new(new_child.clone()),
                    });
                }
            } else {
                child_patches.push(ChildPatchOp::Insert {
                    index: new_idx,
                    node: Box::new(new_child.clone()),
                });
            }
        }

        for (idx, child) in old_children.iter().enumerate() {
            if !matched_old_indices.contains(&idx) {
                child_patches.push(ChildPatchOp::Remove {
                    node_id: child.node_id(),
                    index: idx,
                });
            }
        }
    } else {
        let max_len = std::cmp::max(old_children.len(), new_children.len());
        for i in 0..max_len {
            match (old_children.get(i), new_children.get(i)) {
                (Some(old_child), Some(new_child)) => {
                    let p = diff_nodes_fn(old_child, new_child);
                    if !p.is_no_change() {
                        child_patches.push(ChildPatchOp::Update {
                            node_id: old_child.node_id(),
                            patch: Box::new(p),
                        });
                    }
                }
                (None, Some(new_child)) => {
                    child_patches.push(ChildPatchOp::Insert {
                        index: i,
                        node: Box::new(new_child.clone()),
                    });
                }
                (Some(old_child), None) => {
                    child_patches.push(ChildPatchOp::Remove {
                        node_id: old_child.node_id(),
                        index: i,
                    });
                }
                (None, None) => {}
            }
        }
    }

    if child_patches.is_empty() {
        Patch::NoChange
    } else {
        tracing::trace!(
            parent_id = %parent_id,
            patch_count = child_patches.len(),
            "Child reconciliation produced patches"
        );
        Patch::UpdateChildren {
            node_id: parent_id,
            child_patches,
        }
    }
}
