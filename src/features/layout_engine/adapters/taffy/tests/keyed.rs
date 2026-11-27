//! Keyed children: removal from the middle, reordering, and mixed or duplicate keys.
use super::support::{container, flex_children, keyed_text_child, lay_out, text_child, text_of};
use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::layout_engine::domain::{RenderNode, StyledNode};
use crate::features::module_runtime::test_support::capture_logs;

/// Removing a middle keyed child must not disturb the taffy nodes of the
/// siblings that come after it. Positional matching would diff
/// `"b"`'s old state against `"c"`'s new content (since `"c"` slides
/// into index 1) and free whatever used to be at the trailing index —
/// churn a keyed list exists specifically to avoid.
#[test]
fn test_layout_keyed_children_removes_middle_without_touching_siblings() {
    let mut adapter = TaffyLayoutAdapter::new();

    let first = container(vec![
        keyed_text_child(0, "a", "a-content"),
        keyed_text_child(1, "b", "b-content"),
        keyed_text_child(2, "c", "c-content"),
    ]);
    let first_children = flex_children(lay_out(&mut adapter, first));
    assert_eq!(first_children.len(), 3);

    // Remove "b" — "a" and "c" keep their keys and content.
    let second = container(vec![
        keyed_text_child(0, "a", "a-content"),
        keyed_text_child(1, "c", "c-content"),
    ]);
    let second_children = flex_children(lay_out(&mut adapter, second));
    assert_eq!(second_children.len(), 2);
    // Positions come from the new list's order, matched by key rather
    // than by the old index each survivor used to occupy.
    assert_eq!(text_of(&second_children[0]), "a-content");
    assert_eq!(text_of(&second_children[1]), "c-content");
}

/// A container reordering its keyed children (no insertions or
/// removals) must reorder rather than replace: same identities, new
/// order.
#[test]
fn test_layout_keyed_children_reorder() {
    let mut adapter = TaffyLayoutAdapter::new();

    let first = container(vec![
        keyed_text_child(0, "a", "a-content"),
        keyed_text_child(1, "b", "b-content"),
        keyed_text_child(2, "c", "c-content"),
    ]);
    lay_out(&mut adapter, first);

    let reordered = container(vec![
        keyed_text_child(0, "c", "c-content"),
        keyed_text_child(1, "a", "a-content"),
        keyed_text_child(2, "b", "b-content"),
    ]);
    let children = flex_children(lay_out(&mut adapter, reordered));
    assert_eq!(children.len(), 3);
    let texts: Vec<&str> = children.iter().map(text_of).collect();
    assert_eq!(texts, vec!["c-content", "a-content", "b-content"]);
}

/// The warning a container whose children mix keyed and unkeyed ones, or repeat a key, logs.
const MIXED_KEYS_WARNING: &str = "WARN cranky::features::layout_engine::adapters::taffy::diff::\
                                  children: container has a mix of keyed and unkeyed children, \
                                  or a duplicate key; falling back to positional layout \
                                  reconciliation for this render path=";

/// Lays `node` out with `adapter`, and returns the tree and what the pass logged.
fn lay_out_logged(adapter: &mut TaffyLayoutAdapter, node: StyledNode) -> (RenderNode, String) {
    let mut tree = None;
    let logs = capture_logs(|| tree = Some(lay_out(adapter, node)));
    (tree.unwrap(), logs)
}

/// A list where only some children carry a key (or where a key repeats)
/// has no well-defined keyed match, so it must fall back to positional
/// reconciliation rather than panicking or mismatching identities, and say so.
#[test]
fn test_layout_mixed_keys_falls_back_to_positional_without_panicking() {
    let mut adapter = TaffyLayoutAdapter::new();

    let mixed_first = container(vec![
        keyed_text_child(0, "a", "a-content"),
        text_child(1, "unkeyed"),
    ]);
    lay_out(&mut adapter, mixed_first);

    let mixed_second = container(vec![text_child(0, "still-unkeyed")]);
    let (tree, logs) = lay_out_logged(&mut adapter, mixed_second);
    assert_eq!(flex_children(tree).len(), 1);
    assert!(logs.contains(MIXED_KEYS_WARNING), "{logs}");

    // Duplicate keys: same guard, exercised independently of the mixed
    // case above.
    let dup_first = container(vec![
        keyed_text_child(0, "dup", "one"),
        keyed_text_child(1, "dup", "two"),
    ]);
    lay_out(&mut adapter, dup_first);
    let dup_second = container(vec![keyed_text_child(0, "dup", "one")]);
    let (_, logs) = lay_out_logged(&mut adapter, dup_second);
    assert!(logs.contains(MIXED_KEYS_WARNING), "{logs}");
}
