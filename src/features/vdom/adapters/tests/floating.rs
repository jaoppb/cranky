//! Popup and panel diffing: a content change is patched, a placement change with unchanged
//! content replaces the content under a fresh id, and an added or removed sub-node is replaced.
use super::{diff, text};
use crate::features::vdom::adapters::DefaultVdomDiffAdapter;
use crate::features::vdom::domain::{
    AnchorDirection, ElementSelectors, ExclusiveZone, Interactions, KeyboardInteractivity, NodeId,
    PanelAnchor, PanelLayer, PanelSpec, Patch, PopupOffset, PopupSpec, VNode,
};
use crate::shared::config::domain::{AxisMargins, MarginConfig, MarginOffset};

/// A text button opening `popup`.
fn with_popup(popup: PopupSpec) -> VNode {
    text("button").with_popup(popup)
}

/// A text button opening `panel`.
fn with_panel(panel: PanelSpec) -> VNode {
    text("button").with_panel(panel)
}

/// The node a `Replace` patch installs, after checking it replaces a fresh id rather than
/// `previous`.
fn replaced_under_fresh_id(patch: Option<Box<Patch>>, previous: NodeId) -> VNode {
    let Some(Patch::Replace {
        old_node_id,
        new_node,
    }) = patch.map(|boxed| *boxed)
    else {
        panic!("expected a Replace patch");
    };
    assert_ne!(old_node_id, previous);
    *new_node
}

#[test]
fn popup_without_any_change_has_no_patch() {
    let adapter = DefaultVdomDiffAdapter::new();
    let node = with_popup(PopupSpec::new(Box::new(text("pop"))));
    assert_eq!(adapter.diff_popup(&text("a"), &text("a")), None);
    assert_eq!(adapter.diff_popup(&node, &node.clone()), None);
}

#[test]
fn popup_content_change_with_same_placement_is_the_content_patch() {
    let adapter = DefaultVdomDiffAdapter::new();
    let content = text("old");
    let old = with_popup(PopupSpec::new(Box::new(content.clone())));
    let new = with_popup(PopupSpec::new(Box::new(text("new"))));
    assert_eq!(
        adapter.diff_popup(&old, &new),
        Some(Box::new(diff(&content, &text("new"))))
    );
}

#[test]
fn popup_placement_change_with_same_content_replaces_the_content_under_a_fresh_id() {
    let adapter = DefaultVdomDiffAdapter::new();
    let content = text("pop");
    let spec = PopupSpec::new(Box::new(content.clone()));
    let old = with_popup(spec.clone());
    for new_spec in [
        spec.clone().with_anchor(AnchorDirection::Top),
        spec.with_offset(Some(PopupOffset::new(1, 2))),
    ] {
        let patch = adapter.diff_popup(&old, &with_popup(new_spec));
        assert_eq!(replaced_under_fresh_id(patch, content.node_id()), content);
    }
}

#[test]
fn popup_placement_and_content_change_is_the_content_patch() {
    let adapter = DefaultVdomDiffAdapter::new();
    let content = text("old");
    let old = with_popup(PopupSpec::new(Box::new(content.clone())));
    let new_spec = PopupSpec::new(Box::new(text("new"))).with_anchor(AnchorDirection::Left);
    assert_eq!(
        adapter.diff_popup(&old, &with_popup(new_spec)),
        Some(Box::new(diff(&content, &text("new"))))
    );
}

#[test]
fn popup_added_or_removed_is_replaced() {
    let adapter = DefaultVdomDiffAdapter::new();
    let content = text("pop");
    let node = with_popup(PopupSpec::new(Box::new(content.clone())));
    let added = adapter.diff_popup(&text("button"), &node);
    assert_eq!(replaced_under_fresh_id(added, content.node_id()), content);
    let Some(Patch::Replace {
        old_node_id,
        new_node,
    }) = adapter
        .diff_popup(&node, &text("button"))
        .map(|boxed| *boxed)
    else {
        panic!("expected a Replace patch");
    };
    assert_eq!(old_node_id, content.node_id());
    assert_eq!(
        *new_node,
        VNode {
            node_id: new_node.node_id(),
            ..VNode::new_rect(ElementSelectors::default(), Interactions::default())
        }
    );
}

#[test]
fn panel_content_change_with_same_placement_is_the_content_patch() {
    let adapter = DefaultVdomDiffAdapter::new();
    let content = text("old");
    let old = with_panel(PanelSpec::new(Box::new(content.clone())));
    let new = with_panel(PanelSpec::new(Box::new(text("new"))));
    assert_eq!(adapter.diff_panel(&old, &old.clone()), None);
    assert_eq!(adapter.diff_panel(&text("a"), &text("a")), None);
    assert_eq!(
        adapter.diff_panel(&old, &new),
        Some(Box::new(diff(&content, &text("new"))))
    );
}

#[test]
fn panel_placement_change_with_same_content_replaces_the_content_under_a_fresh_id() {
    let adapter = DefaultVdomDiffAdapter::new();
    let content = text("panel");
    let spec = PanelSpec::new(Box::new(content.clone()));
    let old = with_panel(spec.clone());
    let margin = MarginConfig::new(
        AxisMargins::both(MarginOffset::new(3)),
        AxisMargins::both(MarginOffset::new(0)),
    );
    for new_spec in [
        spec.clone().with_layer(PanelLayer::Overlay),
        spec.clone().with_anchor(PanelAnchor::all()),
        spec.clone().with_margin(margin),
        spec.clone().with_exclusive_zone(ExclusiveZone::auto()),
        spec.with_keyboard(KeyboardInteractivity::Exclusive),
    ] {
        let patch = adapter.diff_panel(&old, &with_panel(new_spec));
        assert_eq!(replaced_under_fresh_id(patch, content.node_id()), content);
    }
}

#[test]
fn panel_placement_and_content_change_is_the_content_patch() {
    let adapter = DefaultVdomDiffAdapter::new();
    let content = text("old");
    let old = with_panel(PanelSpec::new(Box::new(content.clone())));
    let new_spec = PanelSpec::new(Box::new(text("new"))).with_layer(PanelLayer::Bottom);
    assert_eq!(
        adapter.diff_panel(&old, &with_panel(new_spec)),
        Some(Box::new(diff(&content, &text("new"))))
    );
}

#[test]
fn panel_added_or_removed_is_replaced() {
    let adapter = DefaultVdomDiffAdapter::new();
    let content = text("panel");
    let node = with_panel(PanelSpec::new(Box::new(content.clone())));
    let added = adapter.diff_panel(&text("button"), &node);
    assert_eq!(replaced_under_fresh_id(added, content.node_id()), content);
    let Some(Patch::Replace { old_node_id, .. }) = adapter
        .diff_panel(&node, &text("button"))
        .map(|boxed| *boxed)
    else {
        panic!("expected a Replace patch");
    };
    assert_eq!(old_node_id, content.node_id());
}
