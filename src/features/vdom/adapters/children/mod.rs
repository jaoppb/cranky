//! Child-list diffing: keyed when every sibling on both sides carries a unique key, positional
//! otherwise.
use crate::features::vdom::domain::{ChildPatchOp, NodeId, Patch, VNode};
use keyed::keyed_patches;
use std::collections::HashSet;

mod keyed;

/// Whether every child in `children` carries a key, with no duplicates.
/// Vacuously true for an empty slice — callers must separately check that at
/// least one child is keyed before treating that as "safe to key this list".
fn fully_keyed(children: &[VNode]) -> bool {
    let mut seen = HashSet::new();
    children
        .iter()
        .all(|child| child.key().is_some_and(|key| seen.insert(key.as_str())))
}

/// The two child lists of one parent being reconciled: the children it had and the ones it has
/// now.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ChildLists<'a> {
    old: &'a [VNode],
    new: &'a [VNode],
}

impl<'a> ChildLists<'a> {
    #[must_use]
    pub(crate) const fn new(old: &'a [VNode], new: &'a [VNode]) -> Self {
        Self { old, new }
    }
}

#[must_use]
pub(crate) fn diff_children_with<F>(
    parent_id: NodeId,
    lists: ChildLists<'_>,
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
    let any_keyed = lists.old.iter().any(|child| child.key().is_some())
        || lists.new.iter().any(|child| child.key().is_some());
    let keyed = any_keyed && fully_keyed(lists.old) && fully_keyed(lists.new);

    if any_keyed && !keyed {
        log_positional_fallback(parent_id);
    }

    let child_patches = if keyed {
        keyed_patches(lists, &diff_nodes_fn)
    } else {
        positional_patches(lists, &diff_nodes_fn)
    };
    children_patch(parent_id, child_patches)
}

/// Warns that a child list with a mix of keyed and unkeyed children, or a duplicate key, is
/// reconciled by position this render.
fn log_positional_fallback(parent_id: NodeId) {
    tracing::warn!(
        parent_id = %parent_id,
        "child list has a mix of keyed and unkeyed children, or a duplicate key; \
         falling back to positional reconciliation for this render"
    );
}

/// Matches the children by position: shared positions are updated, the longer list's tail
/// inserted or removed.
fn positional_patches<F>(lists: ChildLists<'_>, diff_nodes_fn: &F) -> Vec<ChildPatchOp>
where
    F: Fn(&VNode, &VNode) -> Patch,
{
    let mut child_patches = Vec::new();
    let max_len = std::cmp::max(lists.old.len(), lists.new.len());
    for index in 0..max_len {
        match (lists.old.get(index), lists.new.get(index)) {
            (Some(old_child), Some(new_child)) => {
                let patch = diff_nodes_fn(old_child, new_child);
                push_update(&mut child_patches, old_child, patch);
            }
            (None, Some(new_child)) => child_patches.push(insert_op(index, new_child)),
            (Some(old_child), None) => child_patches.push(remove_op(old_child, index)),
            (None, None) => {}
        }
    }
    child_patches
}

/// Pushes an `Update` of `old_child` with `patch`, unless the patch is `NoChange`.
fn push_update(child_patches: &mut Vec<ChildPatchOp>, old_child: &VNode, patch: Patch) {
    if !patch.is_no_change() {
        child_patches.push(ChildPatchOp::Update {
            node_id: old_child.node_id(),
            patch: Box::new(patch),
        });
    }
}

/// Inserts a copy of `new_child` at `index`.
fn insert_op(index: usize, new_child: &VNode) -> ChildPatchOp {
    ChildPatchOp::Insert {
        index,
        node: Box::new(new_child.clone()),
    }
}

/// Removes `old_child` from `index`.
const fn remove_op(old_child: &VNode, index: usize) -> ChildPatchOp {
    ChildPatchOp::Remove {
        node_id: old_child.node_id(),
        index,
    }
}

/// `NoChange` when no child changed, else the parent's `UpdateChildren`, traced with its count.
fn children_patch(parent_id: NodeId, child_patches: Vec<ChildPatchOp>) -> Patch {
    if child_patches.is_empty() {
        Patch::NoChange
    } else {
        log_child_patches(parent_id, child_patches.len());
        Patch::UpdateChildren {
            node_id: parent_id,
            child_patches,
        }
    }
}

/// Traces how many child patches the reconciliation of `parent_id`'s children produced.
fn log_child_patches(parent_id: NodeId, patch_count: usize) {
    tracing::trace!(
        parent_id = %parent_id,
        patch_count,
        "Child reconciliation produced patches"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::vdom::domain::{ElementSelectors, Interactions, NodeKey, TextContent};

    fn text(content: &str) -> VNode {
        VNode::new_text(
            TextContent::new(content.to_string()),
            ElementSelectors::default(),
            Interactions::default(),
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
        match diff_children_with(parent, ChildLists::new(old, new), replace_if_different) {
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
            diff_children_with(
                parent,
                ChildLists::new(&children, &children),
                replace_if_different
            ),
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
