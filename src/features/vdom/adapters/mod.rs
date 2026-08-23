use crate::features::vdom::domain::{ChildPatchOp, DiffResult, NodeId, Patch, VNode, VNodeKind};
use crate::features::vdom::ports::VdomDiffPort;
use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct DefaultVdomDiffAdapter;

impl DefaultVdomDiffAdapter {
    pub fn new() -> Self {
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

        let tooltip_patch = match (old_node.tooltip(), new_node.tooltip()) {
            (None, None) => None,
            (Some(old_tt), Some(new_tt)) => {
                let p = self.diff_nodes(old_tt, new_tt);
                if p.is_no_change() {
                    None
                } else {
                    Some(Box::new(p))
                }
            }
            (None, Some(new_tt)) => Some(Box::new(Patch::Replace {
                old_node_id: NodeId::new(),
                new_node: Box::new(new_tt.clone()),
            })),
            (Some(old_tt), None) => Some(Box::new(Patch::Replace {
                old_node_id: old_tt.node_id(),
                new_node: Box::new(VNode::new_rect(None, None, None, None, None)),
            })),
        };

        let kind_patch = match (old_node.kind(), new_node.kind()) {
            (VNodeKind::Text { text: old_text }, VNodeKind::Text { text: new_text }) => {
                if old_text != new_text {
                    Patch::UpdateText {
                        node_id: old_node.node_id(),
                        new_text: new_text.clone(),
                    }
                } else {
                    Patch::NoChange
                }
            }
            (
                VNodeKind::Progress {
                    value: old_val,
                    orientation: old_orient,
                },
                VNodeKind::Progress {
                    value: new_val,
                    orientation: new_orient,
                },
            ) => {
                if old_val != new_val || old_orient != new_orient {
                    Patch::UpdateProgress {
                        node_id: old_node.node_id(),
                        new_value: *new_val,
                        new_orientation: *new_orient,
                    }
                } else {
                    Patch::NoChange
                }
            }
            (
                VNodeKind::Image {
                    data: old_data,
                    pixel_size: old_size,
                },
                VNodeKind::Image {
                    data: new_data,
                    pixel_size: new_size,
                },
            ) => {
                if old_data != new_data || old_size != new_size {
                    Patch::UpdateImage {
                        node_id: old_node.node_id(),
                        new_data: new_data.clone(),
                        new_pixel_size: *new_size,
                    }
                } else {
                    Patch::NoChange
                }
            }
            (VNodeKind::Rect, VNodeKind::Rect) => Patch::NoChange,
            (
                VNodeKind::Flex {
                    children: old_children,
                },
                VNodeKind::Flex {
                    children: new_children,
                },
            ) => self.diff_children(old_node.node_id(), old_children, new_children),
            _ => Patch::Replace {
                old_node_id: old_node.node_id(),
                new_node: Box::new(new_node.clone()),
            },
        };

        let props_dirty =
            class_changed || id_changed || handlers_changed || tooltip_patch.is_some();

        if props_dirty {
            Patch::UpdateProps {
                node_id: old_node.node_id(),
                class_changed,
                id_changed,
                handlers_changed,
                tooltip_patch,
                kind_patch: Box::new(kind_patch),
            }
        } else {
            kind_patch
        }
    }

    fn diff_children(
        &self,
        parent_id: NodeId,
        old_children: &[VNode],
        new_children: &[VNode],
    ) -> Patch {
        let has_keys = old_children.iter().any(|c| c.key().is_some())
            || new_children.iter().any(|c| c.key().is_some());

        let mut child_patches = Vec::new();

        if has_keys {
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
                        let p = self.diff_nodes(old_child, new_child);
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
                if i < old_children.len() && i < new_children.len() {
                    let p = self.diff_nodes(&old_children[i], &new_children[i]);
                    if !p.is_no_change() {
                        child_patches.push(ChildPatchOp::Update {
                            node_id: old_children[i].node_id(),
                            patch: Box::new(p),
                        });
                    }
                } else if i >= old_children.len() {
                    child_patches.push(ChildPatchOp::Insert {
                        index: i,
                        node: Box::new(new_children[i].clone()),
                    });
                } else {
                    child_patches.push(ChildPatchOp::Remove {
                        node_id: old_children[i].node_id(),
                        index: i,
                    });
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
}

impl VdomDiffPort for DefaultVdomDiffAdapter {
    fn diff<'a>(&self, old_tree: Option<&'a VNode>, new_tree: &'a VNode) -> DiffResult {
        match old_tree {
            None => {
                tracing::trace!(
                    new_tag = %new_tree.tag(),
                    "Diffing VDOM: initial render (no previous tree)"
                );
                DiffResult::new(Patch::Replace {
                    old_node_id: new_tree.node_id(),
                    new_node: Box::new(new_tree.clone()),
                })
            }
            Some(old) => {
                let patch = self.diff_nodes(old, new_tree);
                tracing::trace!(
                    old_tag = %old.tag(),
                    new_tag = %new_tree.tag(),
                    is_unchanged = patch.is_no_change(),
                    "Diffing VDOM completed"
                );
                DiffResult::new(patch)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::styling::domain::ClassNameList;
    use crate::features::vdom::domain::{NodeKey, TextContent};

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
        .with_key(k1.clone());
        let child2 = VNode::new_text(
            TextContent::new("2".to_string()),
            None,
            None,
            None,
            None,
            None,
        )
        .with_key(k2.clone());

        let old_tree = VNode::new_flex(
            vec![child1.clone(), child2.clone()],
            None,
            None,
            None,
            None,
            None,
        );
        let new_tree = VNode::new_flex(
            vec![child2.clone(), child1.clone()],
            None,
            None,
            None,
            None,
            None,
        );

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
        let new_tree = VNode::new_flex(
            vec![child1.clone(), child2.clone(), child3.clone()],
            None,
            None,
            None,
            None,
            None,
        );

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
}
