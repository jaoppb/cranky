//! Child reconciliation details the other tests leave open: the order of a moved and changed
//! keyed child's patches, and the warning and trace the reconciliation logs.
use super::{diff, flex, keyed, text};
use crate::features::module_runtime::test_support::capture_logs;
use crate::features::vdom::domain::{ChildPatchOp, Patch};

#[test]
fn a_moved_and_changed_keyed_child_is_moved_before_it_is_updated() {
    let (first, second) = (keyed("a", "1"), keyed("b", "2"));
    let old = flex(vec![first.clone(), second.clone()]);
    let new = flex(vec![keyed("b", "two"), first]);
    let Patch::UpdateChildren { child_patches, .. } = diff(&old, &new) else {
        panic!("expected UpdateChildren");
    };
    let kinds: Vec<_> = child_patches
        .iter()
        .map(|child_op| match child_op {
            ChildPatchOp::Move {
                node_id,
                source,
                target,
            } => format!("move {} {source}->{target}", *node_id == second.node_id()),
            ChildPatchOp::Update { node_id, .. } => {
                format!("update {}", *node_id == second.node_id())
            }
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(kinds, ["move true 1->0", "update true", "move false 0->1"]);
}

#[test]
fn a_mixed_or_duplicate_keyed_list_on_either_side_warns_once_with_the_parent_id() {
    let (mixed, unkeyed) = (vec![keyed("a", "1"), text("2")], vec![text("1"), text("2")]);
    let duplicate = vec![keyed("a", "1"), keyed("a", "2")];
    let unique = vec![keyed("a", "1"), keyed("b", "2")];
    for (old_children, new_children) in [
        (mixed.clone(), unkeyed.clone()),
        (duplicate, unkeyed.clone()),
        (unique.clone(), mixed),
        (unkeyed, unique),
    ] {
        let (old, new) = (flex(old_children), flex(new_children));
        let logs = capture_logs(|| {
            diff(&old, &new);
        });
        let expected = format!(
            "WARN cranky::features::vdom::adapters::children: child list has a mix of keyed and \
             unkeyed children, or a duplicate key; falling back to positional reconciliation for \
             this render parent_id={}",
            old.node_id()
        );
        assert_eq!(logs.matches(&expected).count(), 1, "{logs}");
    }
}

#[test]
fn a_fully_keyed_or_unkeyed_list_does_not_warn() {
    for (old, new) in [
        (vec![keyed("a", "1")], vec![keyed("b", "1")]),
        (vec![text("1")], vec![text("2")]),
    ] {
        let logs = capture_logs(|| {
            diff(&flex(old), &flex(new));
        });
        assert!(!logs.contains("WARN"), "{logs}");
    }
}

#[test]
fn a_reconciliation_with_patches_traces_the_parent_and_count_and_one_without_stays_silent() {
    let old = flex(vec![text("1"), text("2")]);
    let new = flex(vec![text("a")]);
    let logs = capture_logs(|| {
        diff(&old, &new);
        diff(&old, &old.clone());
    });
    let expected = format!(
        "TRACE cranky::features::vdom::adapters::children: Child reconciliation produced \
         patches parent_id={} patch_count=2",
        old.node_id()
    );
    assert_eq!(logs.matches("Child reconciliation").count(), 1, "{logs}");
    assert!(logs.contains(&expected), "{logs}");
}
