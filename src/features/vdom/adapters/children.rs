use crate::features::vdom::domain::{ChildPatchOp, NodeId, Patch, VNode};
use std::collections::{HashMap, HashSet};

/// Whether every child in `children` carries a key, with no duplicates.
/// Vacuously true for an empty slice — callers must separately check that at
/// least one child is keyed before treating that as "safe to key this list".
fn fully_keyed(children: &[VNode]) -> bool {
    let mut seen = HashSet::new();
    children
        .iter()
        .all(|child| child.key().is_some_and(|key| seen.insert(key.as_str())))
}

#[must_use]
pub(crate) fn diff_children_with<F>(
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
    let any_keyed = old_children.iter().any(|child| child.key().is_some())
        || new_children.iter().any(|child| child.key().is_some());
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
                            source: old_idx,
                            target: new_idx,
                        });
                    }
                    let patch = diff_nodes_fn(old_child, new_child);
                    if !patch.is_no_change() {
                        child_patches.push(ChildPatchOp::Update {
                            node_id: old_child.node_id(),
                            patch: Box::new(patch),
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
        for index in 0..max_len {
            match (old_children.get(index), new_children.get(index)) {
                (Some(old_child), Some(new_child)) => {
                    let patch = diff_nodes_fn(old_child, new_child);
                    if !patch.is_no_change() {
                        child_patches.push(ChildPatchOp::Update {
                            node_id: old_child.node_id(),
                            patch: Box::new(patch),
                        });
                    }
                }
                (None, Some(new_child)) => {
                    child_patches.push(ChildPatchOp::Insert {
                        index,
                        node: Box::new(new_child.clone()),
                    });
                }
                (Some(old_child), None) => {
                    child_patches.push(ChildPatchOp::Remove {
                        node_id: old_child.node_id(),
                        index,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::vdom::domain::{NodeKey, TextContent};

    fn text(content: &str) -> VNode {
        VNode::new_text(
            TextContent::new(content.to_string()),
            None,
            None,
            None,
            None,
            None,
        )
    }

    fn keyed(key: &str, content: &str) -> VNode {
        text(content).with_key(NodeKey::new(key).unwrap())
    }

    /// Stands in for the node differ: equal nodes are unchanged, anything
    /// else is replaced.
    fn replace_if_different(old: &VNode, new: &VNode) -> Patch {
        if old == new {
            Patch::NoChange
        } else {
            Patch::Replace {
                old_node_id: old.node_id(),
                new_node: Box::new(new.clone()),
            }
        }
    }

    fn child_patches(old: &[VNode], new: &[VNode]) -> Vec<ChildPatchOp> {
        let parent = text("parent").node_id();
        match diff_children_with(parent, old, new, replace_if_different) {
            Patch::UpdateChildren {
                node_id,
                child_patches,
            } => {
                assert_eq!(node_id, parent);
                child_patches
            }
            Patch::NoChange => Vec::new(),
            other => panic!("unexpected patch: {other:?}"),
        }
    }

    #[test]
    fn identical_lists_produce_no_change() {
        let children = [keyed("a", "1"), keyed("b", "2")];
        let parent = text("parent").node_id();
        assert_eq!(
            diff_children_with(parent, &children, &children, replace_if_different),
            Patch::NoChange
        );
    }

    #[test]
    fn keyed_lists_move_matches_insert_new_keys_and_remove_missing_ones() {
        let (first, second, third) = (keyed("a", "1"), keyed("b", "2"), keyed("c", "3"));
        let fourth = keyed("d", "4");
        let old = [first.clone(), second.clone(), third.clone()];
        let new = [third.clone(), first.clone(), fourth.clone()];

        assert_eq!(
            child_patches(&old, &new),
            vec![
                ChildPatchOp::Move {
                    node_id: third.node_id(),
                    source: 2,
                    target: 0,
                },
                ChildPatchOp::Move {
                    node_id: first.node_id(),
                    source: 0,
                    target: 1,
                },
                ChildPatchOp::Insert {
                    index: 2,
                    node: Box::new(fourth),
                },
                ChildPatchOp::Remove {
                    node_id: second.node_id(),
                    index: 1,
                },
            ]
        );
    }

    #[test]
    fn keyed_match_in_place_is_updated_against_its_old_node_without_a_move() {
        let old = [keyed("a", "1"), keyed("b", "2")];
        let changed = keyed("b", "two");
        let new = [old[0].clone(), changed.clone()];

        assert_eq!(
            child_patches(&old, &new),
            vec![ChildPatchOp::Update {
                node_id: old[1].node_id(),
                patch: Box::new(Patch::Replace {
                    old_node_id: old[1].node_id(),
                    new_node: Box::new(changed),
                }),
            }]
        );
    }

    #[test]
    fn unkeyed_lists_are_matched_by_position_and_the_tail_is_removed() {
        let old = [text("1"), text("2"), text("3")];
        let changed = text("one");
        let new = [changed.clone()];

        assert_eq!(
            child_patches(&old, &new),
            vec![
                ChildPatchOp::Update {
                    node_id: old[0].node_id(),
                    patch: Box::new(Patch::Replace {
                        old_node_id: old[0].node_id(),
                        new_node: Box::new(changed),
                    }),
                },
                ChildPatchOp::Remove {
                    node_id: old[1].node_id(),
                    index: 1,
                },
                ChildPatchOp::Remove {
                    node_id: old[2].node_id(),
                    index: 2,
                },
            ]
        );
    }

    #[test]
    fn unkeyed_lists_insert_the_extra_tail() {
        let old = [text("1")];
        let added = text("2");
        let new = [old[0].clone(), added.clone()];

        assert_eq!(
            child_patches(&old, &new),
            vec![ChildPatchOp::Insert {
                index: 1,
                node: Box::new(added),
            }]
        );
    }
}
