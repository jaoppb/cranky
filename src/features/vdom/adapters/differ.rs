//! `DefaultVdomDiffAdapter`, the `VdomDiffPort` implementation that walks two trees node by node.
use super::children::{ChildLists, diff_children_with};
use super::kind_differ::diff_node_kinds;
use crate::features::vdom::domain::{DiffResult, Patch, VNode, VNodeKind};
use crate::features::vdom::ports::VdomDiffPort;

/// Diffs two `VNode` trees recursively: a changed tag replaces the node, changed props or floating
/// content update it, and children are reconciled by key when every sibling has one.
#[derive(Debug, Default, Clone)]
pub struct DefaultVdomDiffAdapter;

impl DefaultVdomDiffAdapter {
    /// Creates the differ; it holds no state, so one instance can serve every module.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    pub(super) fn diff_nodes(&self, old_node: &VNode, new_node: &VNode) -> Patch {
        if old_node.tag() != new_node.tag() {
            return replace_node(old_node, new_node);
        }
        let attached = self.diff_attached(old_node, new_node);
        let kind_patch = self.diff_kind(old_node, new_node);
        let nodes = NodePair {
            old: old_node,
            new: new_node,
        };
        props_patch(nodes, attached, kind_patch)
    }

    /// The patches of the tooltip, popup and panel the two nodes open, in that order.
    fn diff_attached(&self, old_node: &VNode, new_node: &VNode) -> AttachedPatches {
        AttachedPatches {
            tooltip: self.diff_optional_subnode(old_node.tooltip(), new_node.tooltip()),
            popup: self.diff_popup(old_node, new_node),
            panel: self.diff_panel(old_node, new_node),
        }
    }

    /// The patch for what the nodes hold: their content, or for two flex or two grid containers
    /// their reconciled children.
    fn diff_kind(&self, old_node: &VNode, new_node: &VNode) -> Patch {
        diff_node_kinds(old_node.node_id(), old_node.kind(), new_node.kind()).unwrap_or_else(|| {
            if matches!(
                (old_node.kind(), new_node.kind()),
                (VNodeKind::Flex { .. }, VNodeKind::Flex { .. })
                    | (VNodeKind::Grid { .. }, VNodeKind::Grid { .. })
            ) {
                diff_children_with(
                    old_node.node_id(),
                    ChildLists::new(old_node.children(), new_node.children()),
                    |old_child, new_child| self.diff_nodes(old_child, new_child),
                )
            } else {
                replace_node(old_node, new_node)
            }
        })
    }
}

/// Replaces `old_node` with a copy of `new_node`.
fn replace_node(old_node: &VNode, new_node: &VNode) -> Patch {
    Patch::Replace {
        old_node_id: old_node.node_id(),
        new_node: Box::new(new_node.clone()),
    }
}

/// The previous and the new version of one node, of the same tag.
#[derive(Clone, Copy)]
struct NodePair<'a> {
    old: &'a VNode,
    new: &'a VNode,
}

/// The patches of the tooltip, popup and panel a node opens, each absent when unchanged.
struct AttachedPatches {
    tooltip: Option<Box<Patch>>,
    popup: Option<Box<Patch>>,
    panel: Option<Box<Patch>>,
}

impl AttachedPatches {
    /// Whether the tooltip, popup or panel changed.
    const fn any(&self) -> bool {
        self.tooltip.is_some() || self.popup.is_some() || self.panel.is_some()
    }
}

/// `kind_patch` alone when no prop of the node changed, else an `UpdateProps` that says which
/// props changed and carries the attached patches around `kind_patch`.
fn props_patch(nodes: NodePair<'_>, attached: AttachedPatches, kind_patch: Patch) -> Patch {
    let class_changed = nodes.old.class_names() != nodes.new.class_names();
    let id_changed = nodes.old.element_id() != nodes.new.element_id();
    // Key-only changes must still dirty the tree, or the stale keyless RenderNode is kept.
    let key_changed = nodes.old.key() != nodes.new.key();
    let handlers_changed = handlers_changed(nodes);
    if !(class_changed || id_changed || key_changed || handlers_changed || attached.any()) {
        return kind_patch;
    }
    Patch::UpdateProps {
        node_id: nodes.old.node_id(),
        class_changed,
        id_changed,
        handlers_changed,
        tooltip_patch: attached.tooltip,
        popup_patch: attached.popup,
        panel_patch: attached.panel,
        kind_patch: Box::new(kind_patch),
    }
}

/// Whether the node's click or hover action changed.
fn handlers_changed(nodes: NodePair<'_>) -> bool {
    nodes.old.on_click() != nodes.new.on_click() || nodes.old.on_hover() != nodes.new.on_hover()
}

impl VdomDiffPort for DefaultVdomDiffAdapter {
    fn diff(&self, old_tree: Option<&VNode>, new_tree: &VNode) -> DiffResult {
        old_tree.map_or_else(
            || {
                tracing::trace!(
                    new_tag = %new_tree.tag(),
                    "Diffing VDOM: initial render (no previous tree)"
                );
                DiffResult::new(Patch::Replace {
                    old_node_id: new_tree.node_id(),
                    new_node: Box::new(new_tree.clone()),
                })
            },
            |old| {
                let patch = self.diff_nodes(old, new_tree);
                tracing::trace!(
                    old_tag = %old.tag(),
                    new_tag = %new_tree.tag(),
                    is_unchanged = patch.is_no_change(),
                    "Diffing VDOM completed"
                );
                DiffResult::new(patch)
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::styling::domain::ClassNameList;
    use crate::features::vdom::adapters::tests::{flex, keyed, text};
    use crate::features::vdom::domain::{
        ChildPatchOp, ElementSelectors, Interactions, PopupSpec, TextContent,
    };
    use std::collections::HashMap;

    #[test]
    fn test_diff_initial_render_produces_replace() {
        let adapter = DefaultVdomDiffAdapter::new();
        let new_node = VNode::new_text(
            TextContent::new("initial".to_string()),
            ElementSelectors::default(),
            Interactions::default(),
        );

        let res = adapter.diff(None, &new_node);
        assert!(!res.is_unchanged());
        match res.patch() {
            Patch::Replace {
                new_node: replacement,
                ..
            } => {
                assert_eq!(
                    replacement.tag(),
                    crate::features::vdom::domain::NodeTag::Text
                );
            }
            _ => panic!("Expected Replace patch"),
        }
    }

    #[test]
    fn test_diff_identical_trees_produces_no_change() {
        let adapter = DefaultVdomDiffAdapter::new();
        let node = VNode::new_text(
            TextContent::new("static".to_string()),
            ElementSelectors::default(),
            Interactions::default(),
        );

        let res = adapter.diff(Some(&node), &node);
        assert!(res.is_unchanged());
        assert_eq!(res.patch(), &Patch::NoChange);
    }

    #[test]
    fn test_diff_text_change() {
        let adapter = DefaultVdomDiffAdapter::new();
        let node1 = VNode::new_text(
            TextContent::new("hello".to_string()),
            ElementSelectors::default(),
            Interactions::default(),
        );
        let node2 = VNode::new_text(
            TextContent::new("world".to_string()),
            ElementSelectors::default(),
            Interactions::default(),
        );

        let res = adapter.diff(Some(&node1), &node2);
        assert!(!res.is_unchanged());
        match res.patch() {
            Patch::UpdateText { new_text, .. } => {
                assert_eq!(new_text.as_str(), "world");
            }
            _ => panic!("Expected UpdateText patch"),
        }
    }

    #[test]
    fn test_diff_class_prop_change() {
        let adapter = DefaultVdomDiffAdapter::new();
        let classes = ClassNameList::parse("active").unwrap();

        let node1 = VNode::new_text(
            TextContent::new("test".to_string()),
            ElementSelectors::default(),
            Interactions::default(),
        );
        let node2 = VNode::new_text(
            TextContent::new("test".to_string()),
            ElementSelectors::new(Some(classes), None),
            Interactions::default(),
        );

        let res = adapter.diff(Some(&node1), &node2);
        assert!(!res.is_unchanged());
        match res.patch() {
            Patch::UpdateProps { class_changed, .. } => {
                assert!(class_changed);
            }
            _ => panic!("Expected UpdateProps patch"),
        }
    }

    #[test]
    fn test_diff_keyed_children_reordering() {
        let adapter = DefaultVdomDiffAdapter::new();
        let child1 = keyed("k1", "1");
        let child2 = keyed("k2", "2");

        let old_tree = flex(vec![child1.clone(), child2.clone()]);
        let new_tree = flex(vec![child2, child1]);

        let res = adapter.diff(Some(&old_tree), &new_tree);
        assert!(!res.is_unchanged());
        match res.patch() {
            Patch::UpdateChildren { child_patches, .. } => {
                assert!(
                    child_patches
                        .iter()
                        .any(|child_patch| matches!(child_patch, ChildPatchOp::Move { .. }))
                );
            }
            _ => panic!("Expected UpdateChildren patch with Move"),
        }
    }

    /// A list with a duplicate key has no well-defined keyed match, so it
    /// must fall back to positional reconciliation (an `Update` per shared
    /// index) rather than letting the last duplicate silently win a key
    /// lookup, which would misattribute the other's identity.
    #[test]
    fn test_diff_duplicate_keys_falls_back_to_positional() {
        let adapter = DefaultVdomDiffAdapter::new();
        let child1 = keyed("dup", "1");
        let child2 = keyed("dup", "2");

        let old_tree = flex(vec![child1.clone(), child2.clone()]);
        // Same two nodes, same duplicate key, but their text content swapped
        // position — a keyed diff would (wrongly) call this unchanged.
        let new_tree = flex(vec![child2, child1]);

        let res = adapter.diff(Some(&old_tree), &new_tree);
        match res.patch() {
            Patch::UpdateChildren { child_patches, .. } => {
                assert!(
                    child_patches
                        .iter()
                        .all(|child_patch| matches!(child_patch, ChildPatchOp::Update { .. })),
                    "expected positional Updates, not a keyed Move: {child_patches:?}"
                );
            }
            _ => panic!("Expected UpdateChildren patch"),
        }
    }

    /// A list where some children are keyed and others aren't has the same
    /// "no well-defined match" problem as a duplicate key.
    #[test]
    fn test_diff_mixed_keyed_and_unkeyed_falls_back_to_positional() {
        let adapter = DefaultVdomDiffAdapter::new();
        let keyed_child = keyed("k1", "1");
        let unkeyed_child = text("2");

        let old_tree = flex(vec![keyed_child.clone(), unkeyed_child.clone()]);
        let new_tree = flex(vec![unkeyed_child, keyed_child]);

        let res = adapter.diff(Some(&old_tree), &new_tree);
        match res.patch() {
            Patch::UpdateChildren { child_patches, .. } => {
                assert!(
                    child_patches
                        .iter()
                        .all(|child_patch| matches!(child_patch, ChildPatchOp::Update { .. })),
                    "expected positional Updates, not a keyed Move: {child_patches:?}"
                );
            }
            _ => panic!("Expected UpdateChildren patch"),
        }
    }

    #[test]
    fn test_diff_positional_children() {
        let adapter = DefaultVdomDiffAdapter::new();
        let (child1, child2, child3) = (text("1"), text("2"), text("3"));

        let old_tree = flex(vec![child1.clone(), child2.clone()]);
        let new_tree = flex(vec![child1, child2, child3]);

        let res = adapter.diff(Some(&old_tree), &new_tree);
        assert!(!res.is_unchanged());
        match res.patch() {
            Patch::UpdateChildren { child_patches, .. } => {
                assert_eq!(child_patches.len(), 1);
                assert!(matches!(
                    child_patches[0],
                    ChildPatchOp::Insert { index: 2, .. }
                ));
            }
            _ => panic!("Expected UpdateChildren patch with Insert"),
        }
    }

    #[test]
    fn test_diff_module_node_change() {
        use crate::features::vdom::domain::ModuleParams;
        use crate::shared::primitives::{DynamicValue, ModuleName, ModuleOptions};

        let adapter = DefaultVdomDiffAdapter::new();
        let clock = |options| {
            let params = ModuleParams::new(ModuleName::new("clock"), None, options);
            VNode::new_module(params, ElementSelectors::default(), Interactions::default())
        };
        let plain_module = clock(ModuleOptions::default());
        let opts_map = HashMap::from([("format".to_string(), DynamicValue::from("%H:%M"))]);
        let formatted_module = clock(ModuleOptions::new(opts_map));

        let unchanged = adapter.diff(Some(&plain_module), &plain_module);
        assert!(unchanged.is_unchanged());

        let changed = adapter.diff(Some(&plain_module), &formatted_module);
        assert!(!changed.is_unchanged());
        assert!(matches!(changed.patch(), Patch::UpdateModule { .. }));
    }

    #[test]
    fn test_diff_popup_node_change() {
        let adapter = DefaultVdomDiffAdapter::new();
        let base = text("btn");

        let popup1 = text("pop1");
        let with_pop1 = base.clone().with_popup(PopupSpec::new(Box::new(popup1)));

        // 1. None -> Some(popup)
        let diff1 = adapter.diff(Some(&base), &with_pop1);
        assert!(!diff1.is_unchanged());
        let Patch::UpdateProps { popup_patch, .. } = diff1.patch() else {
            panic!("Expected UpdateProps with popup_patch");
        };
        assert!(popup_patch.is_some());

        // 2. Some(popup) -> None
        let diff2 = adapter.diff(Some(&with_pop1), &base);
        assert!(!diff2.is_unchanged());
        let Patch::UpdateProps { popup_patch, .. } = diff2.patch() else {
            panic!("Expected UpdateProps with popup_patch");
        };
        assert!(popup_patch.is_some());
    }

    #[test]
    fn test_diff_grid_children() {
        let adapter = DefaultVdomDiffAdapter::new();
        let (child1, child2) = (text("A"), text("B"));
        let grid = |children| {
            VNode::new_grid(
                children,
                ElementSelectors::default(),
                Interactions::default(),
            )
        };

        let grid1 = grid(vec![child1.clone()]);
        let grid2 = grid(vec![child1, child2]);

        let unchanged = adapter.diff(Some(&grid1), &grid1);
        assert!(unchanged.is_unchanged());

        let changed = adapter.diff(Some(&grid1), &grid2);
        assert!(!changed.is_unchanged());
        assert!(matches!(changed.patch(), Patch::UpdateChildren { .. }));
    }

    #[test]
    fn test_diff_key_only_change_on_unkeyed_list_is_not_unchanged() {
        // Adding keys to an unkeyed list falls back to positional
        // reconciliation (the old side isn't fully keyed); the per-node diff
        // must still see the key change or the pipeline skips the rebuild.
        let adapter = DefaultVdomDiffAdapter::new();
        let old_list = flex(vec![text("a"), text("b")]);
        let new_list = flex(vec![keyed("a", "a"), keyed("b", "b")]);

        let res = adapter.diff(Some(&old_list), &new_list);
        let Patch::UpdateChildren { child_patches, .. } = res.patch() else {
            panic!("expected UpdateChildren, got {:?}", res.patch());
        };
        let updated: Vec<_> = child_patches
            .iter()
            .map(|child_op| match child_op {
                ChildPatchOp::Update { node_id, .. } => Some(*node_id),
                _ => None,
            })
            .collect();
        let positional = old_list
            .children()
            .iter()
            .map(|child| Some(child.node_id()));
        assert_eq!(updated, positional.collect::<Vec<_>>());
    }

    #[test]
    fn test_diff_key_value_change_is_not_unchanged() {
        let adapter = DefaultVdomDiffAdapter::new();
        let make = |key: &str| keyed(key, "same");

        let res = adapter.diff(Some(&make("old")), &make("new"));
        assert!(!res.is_unchanged());
    }

    /// Two nodes of different kinds, which `diff_nodes` never passes on since it replaces a
    /// changed tag first, are replaced whole rather than reconciled.
    #[test]
    fn diff_kind_replaces_a_node_whose_kind_changed() {
        let adapter = DefaultVdomDiffAdapter::new();
        let old_node = text("label");
        let new_node = flex(vec![text("label")]);

        let expected = Patch::Replace {
            old_node_id: old_node.node_id(),
            new_node: Box::new(new_node.clone()),
        };
        assert_eq!(adapter.diff_kind(&old_node, &new_node), expected);
        assert_eq!(adapter.diff_kind(&new_node, &new_node), Patch::NoChange);

        // A flex and a grid are both containers, yet still two kinds.
        let grid = VNode::new_grid(
            vec![text("label")],
            ElementSelectors::default(),
            Interactions::default(),
        );
        let expected = Patch::Replace {
            old_node_id: new_node.node_id(),
            new_node: Box::new(grid.clone()),
        };
        assert_eq!(adapter.diff_kind(&new_node, &grid), expected);
    }
}
