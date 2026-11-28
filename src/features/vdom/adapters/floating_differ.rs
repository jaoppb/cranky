use super::differ::DefaultVdomDiffAdapter;
use crate::features::vdom::domain::{
    AnchorDirection, ElementSelectors, Interactions, NodeId, PanelSpec, PanelSurface, Patch,
    PopupOffset, PopupSpec, VNode,
};

/// Where a node's popup opens, compared to tell a placement change.
fn popup_placement(node: &VNode) -> Option<(AnchorDirection, Option<PopupOffset>)> {
    node.popup()
        .map(|popup| (popup.anchor_direction(), popup.offset()))
}

/// Where a node's panel surface sits, compared to tell a placement change.
fn panel_placement(node: &VNode) -> Option<PanelSurface> {
    node.panel().map(PanelSpec::surface)
}

/// The new content of a popup or panel whose placement changed, replaced under a fresh id so the
/// surface is rebuilt.
fn rebuilt_content(content: &VNode) -> Patch {
    Patch::Replace {
        old_node_id: NodeId::new(),
        new_node: Box::new(content.clone()),
    }
}

impl DefaultVdomDiffAdapter {
    pub(super) fn diff_popup(&self, old_node: &VNode, new_node: &VNode) -> Option<Box<Patch>> {
        let new_content = new_node.popup().map(PopupSpec::content);
        let content_patch =
            self.diff_optional_subnode(old_node.popup().map(PopupSpec::content), new_content);
        if popup_placement(old_node) == popup_placement(new_node) {
            content_patch
        } else {
            content_patch.or_else(|| new_content.map(|content| Box::new(rebuilt_content(content))))
        }
    }

    pub(super) fn diff_panel(&self, old_node: &VNode, new_node: &VNode) -> Option<Box<Patch>> {
        let new_content = new_node.panel().map(PanelSpec::content);
        let content_patch =
            self.diff_optional_subnode(old_node.panel().map(PanelSpec::content), new_content);
        if panel_placement(old_node) == panel_placement(new_node) {
            content_patch
        } else {
            content_patch.or_else(|| new_content.map(|content| Box::new(rebuilt_content(content))))
        }
    }

    pub(super) fn diff_optional_subnode(
        &self,
        old_sub: Option<&VNode>,
        new_sub: Option<&VNode>,
    ) -> Option<Box<Patch>> {
        match (old_sub, new_sub) {
            (None, None) => None,
            (Some(old), Some(new)) => Some(self.diff_nodes(old, new))
                .filter(|patch| !patch.is_no_change())
                .map(Box::new),
            (None, Some(new)) => Some(Box::new(Patch::Replace {
                old_node_id: NodeId::new(),
                new_node: Box::new(new.clone()),
            })),
            (Some(old), None) => Some(Box::new(Patch::Replace {
                old_node_id: old.node_id(),
                new_node: Box::new(VNode::new_rect(
                    ElementSelectors::default(),
                    Interactions::default(),
                )),
            })),
        }
    }
}
