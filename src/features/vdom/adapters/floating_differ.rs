use super::differ::DefaultVdomDiffAdapter;
use crate::features::vdom::domain::{NodeId, Patch, VNode};

impl DefaultVdomDiffAdapter {
    pub(super) fn diff_popup(&self, old_node: &VNode, new_node: &VNode) -> Option<Box<Patch>> {
        let popup_props_changed = old_node
            .popup()
            .map(|popup| (popup.anchor_direction(), popup.offset()))
            != new_node
                .popup()
                .map(|popup| (popup.anchor_direction(), popup.offset()));
        let popup_content_patch = self.diff_optional_subnode(
            old_node
                .popup()
                .map(crate::features::vdom::domain::PopupSpec::content),
            new_node
                .popup()
                .map(crate::features::vdom::domain::PopupSpec::content),
        );
        if popup_props_changed {
            popup_content_patch.or_else(|| {
                new_node.popup().map(|popup| {
                    Box::new(Patch::Replace {
                        old_node_id: NodeId::new(),
                        new_node: Box::new(popup.content().clone()),
                    })
                })
            })
        } else {
            popup_content_patch
        }
    }

    pub(super) fn diff_panel(&self, old_node: &VNode, new_node: &VNode) -> Option<Box<Patch>> {
        let panel_props_changed = old_node.panel().map(|panel| {
            (
                panel.layer(),
                panel.anchor(),
                *panel.margin(),
                panel.exclusive_zone(),
                panel.keyboard(),
            )
        }) != new_node.panel().map(|panel| {
            (
                panel.layer(),
                panel.anchor(),
                *panel.margin(),
                panel.exclusive_zone(),
                panel.keyboard(),
            )
        });
        let panel_content_patch = self.diff_optional_subnode(
            old_node
                .panel()
                .map(crate::features::vdom::domain::PanelSpec::content),
            new_node
                .panel()
                .map(crate::features::vdom::domain::PanelSpec::content),
        );
        if panel_props_changed {
            panel_content_patch.or_else(|| {
                new_node.panel().map(|panel| {
                    Box::new(Patch::Replace {
                        old_node_id: NodeId::new(),
                        new_node: Box::new(panel.content().clone()),
                    })
                })
            })
        } else {
            panel_content_patch
        }
    }

    pub(super) fn diff_optional_subnode(
        &self,
        old_sub: Option<&VNode>,
        new_sub: Option<&VNode>,
    ) -> Option<Box<Patch>> {
        match (old_sub, new_sub) {
            (None, None) => None,
            (Some(old), Some(new)) => {
                let patch = self.diff_nodes(old, new);
                if patch.is_no_change() {
                    None
                } else {
                    Some(Box::new(patch))
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
