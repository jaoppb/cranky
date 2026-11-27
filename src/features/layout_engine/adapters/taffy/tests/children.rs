//! Containers whose children empty out and come back, at the root and one level down.
use super::support::{child_path, container, flex_children, flex_node, lay_out, text_child};
use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::layout_engine::domain::StyledNode;
use crate::features::styling::domain::ComputedStyle;

/// A container that empties and then repopulates must not resurrect the
/// taffy nodes it freed on the way down.
///
/// `workspace.lua` renders exactly this shape: an unconditional
/// `ui.flex` whose children vanish when the monitor it is rendering for
/// owns no workspaces. Before the fix, the emptying pass freed every
/// child node but stored the freed `LayoutState`s back into `self.state`,
/// and the following pass handed one of those dead ids to
/// `remove_recursive` -> `TaffyTree::children` -> `SlotMap` index panic
/// ("invalid `SlotMap` key used").
#[test]
fn test_layout_children_empty_then_repopulated() {
    let mut adapter = TaffyLayoutAdapter::new();

    let populated = container(vec![
        text_child(0, "one"),
        text_child(1, "two"),
        text_child(2, "three"),
    ]);
    lay_out(&mut adapter, populated);

    // N -> 0: every child node is freed.
    lay_out(&mut adapter, container(vec![]));

    // Still empty: the pass that used to panic on a freed NodeId.
    lay_out(&mut adapter, container(vec![]));

    // 0 -> N: the freed children must not be diffed against.
    let repopulated = container(vec![text_child(0, "one"), text_child(1, "two")]);
    let children = flex_children(lay_out(&mut adapter, repopulated));
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].rect().width(), 30);
    assert_eq!(children[1].rect().width(), 30);
}

/// A root container holding one flex child at index 0 whose text children are `texts`.
fn nested(texts: Vec<&str>) -> StyledNode {
    let children = texts
        .into_iter()
        .enumerate()
        .map(|(index, content)| text_child(index, content))
        .collect();
    container(vec![flex_node(
        child_path(0),
        children,
        ComputedStyle::default(),
    )])
}

/// The same transition one level down, where the emptying container is
/// itself a child being patched rather than the diff root.
#[test]
fn test_layout_nested_children_empty_then_repopulated() {
    let mut adapter = TaffyLayoutAdapter::new();

    for frame in [
        nested(vec!["a", "b"]),
        nested(vec![]),
        nested(vec![]),
        nested(vec!["c"]),
    ] {
        lay_out(&mut adapter, frame);
    }
}
