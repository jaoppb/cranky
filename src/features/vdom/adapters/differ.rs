use super::children::diff_children_with;
use super::kind_differ::diff_node_kinds;
use crate::features::vdom::domain::{DiffResult, NodeId, Patch, VNode, VNodeKind};
use crate::features::vdom::ports::VdomDiffPort;

#[derive(Debug, Default, Clone)]
pub struct DefaultVdomDiffAdapter;

impl DefaultVdomDiffAdapter {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    fn diff_nodes(&self, old_node: &VNode, new_node: &VNode) -> Patch {
        if old_node.tag() != new_node.tag() {
            return Patch::Replace {
                old_node_id: old_node.node_id(),
                new_node: Box::new(new_node.clone()),
            };
        }

        let class_changed = old_node.class_names() != new_node.class_names();
        let id_changed = old_node.element_id() != new_node.element_id();
        let handlers_changed = old_node.on_click() != new_node.on_click()
            || old_node.on_hover() != new_node.on_hover();

        let tooltip_patch = self.diff_optional_subnode(old_node.tooltip(), new_node.tooltip());
        let popup_patch = self.diff_optional_subnode(old_node.popup(), new_node.popup());

        let kind_patch = diff_node_kinds(old_node.node_id(), old_node.kind(), new_node.kind())
            .unwrap_or_else(|| {
                if matches!(
                    (old_node.kind(), new_node.kind()),
                    (VNodeKind::Flex { .. }, VNodeKind::Flex { .. })
                        | (VNodeKind::Grid { .. }, VNodeKind::Grid { .. })
                ) {
                    diff_children_with(
                        old_node.node_id(),
                        old_node.children(),
                        new_node.children(),
                        |a, b| self.diff_nodes(a, b),
                    )
                } else {
                    Patch::Replace {
                        old_node_id: old_node.node_id(),
                        new_node: Box::new(new_node.clone()),
                    }
                }
            });

        let props_dirty = class_changed
            || id_changed
            || handlers_changed
            || tooltip_patch.is_some()
            || popup_patch.is_some();

        if props_dirty {
            Patch::UpdateProps {
                node_id: old_node.node_id(),
                class_changed,
                id_changed,
                handlers_changed,
                tooltip_patch,
                popup_patch,
                kind_patch: Box::new(kind_patch),
            }
        } else {
            kind_patch
        }
    }

    fn diff_optional_subnode(
        &self,
        old_sub: Option<&VNode>,
        new_sub: Option<&VNode>,
    ) -> Option<Box<Patch>> {
        match (old_sub, new_sub) {
            (None, None) => None,
            (Some(old), Some(new)) => {
                let p = self.diff_nodes(old, new);
                if p.is_no_change() {
                    None
                } else {
                    Some(Box::new(p))
                }
            }
            (None, Some(new)) => Some(Box::new(Patch::Replace {
                old_node_id: NodeId::new(),
                new_node: Box::new(new.clone()),
            })),
            (Some(old), None) => Some(Box::new(Patch::Replace {
                old_node_id: old.node_id(),
                new_node: Box::new(VNode::new_rect(None, None, None, None, None)),
            })),
        }
    }
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
    use crate::features::vdom::domain::{ChildPatchOp, NodeKey, TextContent};
    use std::collections::HashMap;

    #[test]
    fn test_diff_initial_render_produces_replace() {
        let adapter = DefaultVdomDiffAdapter::new();
        let new_node = VNode::new_text(
            TextContent::new("initial".to_string()),
            None,
            None,
            None,
            None,
            None,
        );

        let res = adapter.diff(None, &new_node);
        assert!(!res.is_unchanged());
        match res.patch() {
            Patch::Replace { new_node: n, .. } => {
                assert_eq!(n.tag(), crate::features::vdom::domain::NodeTag::Text);
            }
            _ => panic!("Expected Replace patch"),
        }
    }

    #[test]
    fn test_diff_identical_trees_produces_no_change() {
        let adapter = DefaultVdomDiffAdapter::new();
        let node = VNode::new_text(
            TextContent::new("static".to_string()),
            None,
            None,
            None,
            None,
            None,
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
            None,
            None,
            None,
            None,
            None,
        );
        let node2 = VNode::new_text(
            TextContent::new("world".to_string()),
            None,
            None,
            None,
            None,
            None,
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
            None,
            None,
            None,
            None,
            None,
        );
        let node2 = VNode::new_text(
            TextContent::new("test".to_string()),
            Some(classes),
            None,
            None,
            None,
            None,
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
        let k1 = NodeKey::new("k1").unwrap();
        let k2 = NodeKey::new("k2").unwrap();

        let child1 = VNode::new_text(
            TextContent::new("1".to_string()),
            None,
            None,
            None,
            None,
            None,
        )
        .with_key(k1);
        let child2 = VNode::new_text(
            TextContent::new("2".to_string()),
            None,
            None,
            None,
            None,
            None,
        )
        .with_key(k2);

        let old_tree = VNode::new_flex(
            vec![child1.clone(), child2.clone()],
            None,
            None,
            None,
            None,
            None,
        );
        let new_tree = VNode::new_flex(vec![child2, child1], None, None, None, None, None);

        let res = adapter.diff(Some(&old_tree), &new_tree);
        assert!(!res.is_unchanged());
        match res.patch() {
            Patch::UpdateChildren { child_patches, .. } => {
                assert!(
                    child_patches
                        .iter()
                        .any(|p| matches!(p, ChildPatchOp::Move { .. }))
                );
            }
            _ => panic!("Expected UpdateChildren patch with Move"),
        }
    }

    #[test]
    fn test_diff_positional_children() {
        let adapter = DefaultVdomDiffAdapter::new();
        let child1 = VNode::new_text(
            TextContent::new("1".to_string()),
            None,
            None,
            None,
            None,
            None,
        );
        let child2 = VNode::new_text(
            TextContent::new("2".to_string()),
            None,
            None,
            None,
            None,
            None,
        );
        let child3 = VNode::new_text(
            TextContent::new("3".to_string()),
            None,
            None,
            None,
            None,
            None,
        );

        let old_tree = VNode::new_flex(
            vec![child1.clone(), child2.clone()],
            None,
            None,
            None,
            None,
            None,
        );
        let new_tree = VNode::new_flex(vec![child1, child2, child3], None, None, None, None, None);

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
        use crate::shared::primitives::{DynamicValue, ModuleOptions};

        let adapter = DefaultVdomDiffAdapter::new();
        let m1 = VNode::new_module(
            crate::shared::primitives::ModuleName::new("clock"),
            None,
            ModuleOptions::default(),
            None,
            None,
            None,
            None,
            None,
        );
        let mut opts_map = HashMap::new();
        opts_map.insert("format".to_string(), DynamicValue::from("%H:%M"));
        let m2 = VNode::new_module(
            crate::shared::primitives::ModuleName::new("clock"),
            None,
            ModuleOptions::new(opts_map),
            None,
            None,
            None,
            None,
            None,
        );

        let unchanged = adapter.diff(Some(&m1), &m1);
        assert!(unchanged.is_unchanged());

        let changed = adapter.diff(Some(&m1), &m2);
        assert!(!changed.is_unchanged());
        assert!(matches!(changed.patch(), Patch::UpdateModule { .. }));
    }

    #[test]
    fn test_diff_popup_node_change() {
        let adapter = DefaultVdomDiffAdapter::new();
        let base = VNode::new_text(
            TextContent::new("btn".to_string()),
            None,
            None,
            None,
            None,
            None,
        );

        let popup1 = VNode::new_text(
            TextContent::new("pop1".to_string()),
            None,
            None,
            None,
            None,
            None,
        );
        let with_pop1 = base.clone().with_popup(Box::new(popup1));

        // 1. None -> Some(popup)
        let diff1 = adapter.diff(Some(&base), &with_pop1);
        assert!(!diff1.is_unchanged());
        match diff1.patch() {
            Patch::UpdateProps { popup_patch, .. } => {
                assert!(popup_patch.is_some());
            }
            _ => panic!("Expected UpdateProps with popup_patch"),
        }

        // 2. Some(popup) -> None
        let diff2 = adapter.diff(Some(&with_pop1), &base);
        assert!(!diff2.is_unchanged());
        match diff2.patch() {
            Patch::UpdateProps { popup_patch, .. } => {
                assert!(popup_patch.is_some());
            }
            _ => panic!("Expected UpdateProps with popup_patch"),
        }
    }

    #[test]
    fn test_diff_grid_children() {
        let adapter = DefaultVdomDiffAdapter::new();
        let child1 = VNode::new_text(
            TextContent::new("A".to_string()),
            None,
            None,
            None,
            None,
            None,
        );
        let child2 = VNode::new_text(
            TextContent::new("B".to_string()),
            None,
            None,
            None,
            None,
            None,
        );

        let grid1 = VNode::new_grid(vec![child1.clone()], None, None, None, None, None);
        let grid2 = VNode::new_grid(vec![child1, child2], None, None, None, None, None);

        let unchanged = adapter.diff(Some(&grid1), &grid1);
        assert!(unchanged.is_unchanged());

        let changed = adapter.diff(Some(&grid1), &grid2);
        assert!(!changed.is_unchanged());
        assert!(matches!(changed.patch(), Patch::UpdateChildren { .. }));
    }
}
