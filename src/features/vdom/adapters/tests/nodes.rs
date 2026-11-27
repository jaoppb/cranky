//! `diff_nodes`: a changed tag replaces the node, changed props wrap the kind patch in an
//! `UpdateProps` that says which props changed, and containers recurse into their children.
use super::props::{Flag, PropsChange, styled, update_props, update_text};
use super::{diff, flex, keyed, text};
use crate::features::styling::domain::{ClassNameList, ElementId};
use crate::features::vdom::domain::{
    ChildPatchOp, ClickHandlers, ElementSelectors, Interactions, PanelSpec, Patch, PopupSpec,
    UiAction, VNode,
};
use crate::shared::events::core::PointerButton;

#[test]
fn a_changed_tag_replaces_the_old_node_with_the_new_one() {
    let (old, new) = (text("a"), flex(vec![]));
    assert_eq!(
        diff(&old, &new),
        Patch::Replace {
            old_node_id: old.node_id(),
            new_node: Box::new(new),
        }
    );
}

#[test]
fn a_class_change_wraps_the_kind_patch() {
    let old = text("a");
    let classes = ClassNameList::parse("active").unwrap();
    let new = styled(
        "b",
        ElementSelectors::new(Some(classes), None),
        Interactions::default(),
    );
    let change = PropsChange {
        flag: Some(Flag::Class),
        ..PropsChange::default()
    };
    assert_eq!(
        diff(&old, &new),
        update_props(old.node_id(), change, update_text(&old, "b"))
    );
}

#[test]
fn an_id_change_is_reported_alone() {
    let old = text("a");
    let element_id = ElementId::new("main").unwrap();
    let new = styled(
        "a",
        ElementSelectors::new(None, Some(element_id)),
        Interactions::default(),
    );
    let change = PropsChange {
        flag: Some(Flag::Identifier),
        ..PropsChange::default()
    };
    assert_eq!(
        diff(&old, &new),
        update_props(old.node_id(), change, Patch::NoChange)
    );
}

#[test]
fn a_key_change_dirties_the_props_without_naming_a_changed_prop() {
    let old = keyed("a", "same");
    assert_eq!(
        diff(&old, &keyed("b", "same")),
        update_props(old.node_id(), PropsChange::default(), Patch::NoChange)
    );
}

#[test]
fn a_click_or_hover_change_is_a_handlers_change() {
    let old = text("a");
    let mut on_click = ClickHandlers::new();
    on_click.insert(PointerButton::Left, UiAction::Exec("run".to_string()));
    let hover = Some(UiAction::Exec("hover".to_string()));
    for interactions in [
        Interactions::new(Some(on_click), None, None),
        Interactions::new(None, hover, None),
    ] {
        let new = styled("a", ElementSelectors::default(), interactions);
        let change = PropsChange {
            flag: Some(Flag::Handlers),
            ..PropsChange::default()
        };
        assert_eq!(
            diff(&old, &new),
            update_props(old.node_id(), change, Patch::NoChange)
        );
    }
}

#[test]
fn tooltip_popup_and_panel_patches_land_in_their_own_fields() {
    let tip = text("tip");
    let with_tooltip = |content: &str| {
        let interactions = Interactions::new(None, None, Some(Box::new(text(content))));
        styled("a", ElementSelectors::default(), interactions)
    };
    let old = with_tooltip("tip")
        .with_popup(PopupSpec::new(Box::new(tip.clone())))
        .with_panel(PanelSpec::new(Box::new(tip.clone())));
    let new = with_tooltip("tip2")
        .with_popup(PopupSpec::new(Box::new(text("pop"))))
        .with_panel(PanelSpec::new(Box::new(text("panel"))));
    let old_tooltip = old.tooltip().unwrap().clone();
    let change = PropsChange {
        tooltip_patch: Some(Box::new(update_text(&old_tooltip, "tip2"))),
        popup_patch: Some(Box::new(update_text(&tip, "pop"))),
        panel_patch: Some(Box::new(update_text(&tip, "panel"))),
        ..PropsChange::default()
    };
    assert_eq!(
        diff(&old, &new),
        update_props(old.node_id(), change, Patch::NoChange)
    );
}

#[test]
fn flex_and_grid_children_are_diffed_recursively() {
    let child = text("a");
    let grid = |children| {
        VNode::new_grid(
            children,
            ElementSelectors::default(),
            Interactions::default(),
        )
    };
    let old_flex = flex(vec![child.clone()]);
    let old_grid = grid(vec![child.clone()]);
    for (old, new) in [
        (&old_flex, flex(vec![text("b")])),
        (&old_grid, grid(vec![text("b")])),
    ] {
        assert_eq!(
            diff(old, &new),
            Patch::UpdateChildren {
                node_id: old.node_id(),
                child_patches: vec![ChildPatchOp::Update {
                    node_id: child.node_id(),
                    patch: Box::new(update_text(&child, "b")),
                }],
            }
        );
    }
}

#[test]
fn a_flex_with_changed_props_and_children_wraps_the_children_patch() {
    let child = text("a");
    let old = flex(vec![child.clone()]);
    let classes = ClassNameList::parse("row").unwrap();
    let new = VNode::new_flex(
        vec![text("b")],
        ElementSelectors::new(Some(classes), None),
        Interactions::default(),
    );
    let children_patch = Patch::UpdateChildren {
        node_id: old.node_id(),
        child_patches: vec![ChildPatchOp::Update {
            node_id: child.node_id(),
            patch: Box::new(update_text(&child, "b")),
        }],
    };
    let change = PropsChange {
        flag: Some(Flag::Class),
        ..PropsChange::default()
    };
    assert_eq!(
        diff(&old, &new),
        update_props(old.node_id(), change, children_patch)
    );
}

#[test]
fn a_tooltip_or_panel_change_alone_dirties_the_props() {
    let tip = |content: &str| Interactions::new(None, None, Some(Box::new(text(content))));
    let old = styled("a", ElementSelectors::default(), tip("tip"));
    let new = styled("a", ElementSelectors::default(), tip("tip2"));
    let change = PropsChange {
        tooltip_patch: Some(Box::new(update_text(old.tooltip().unwrap(), "tip2"))),
        ..PropsChange::default()
    };
    assert_eq!(
        diff(&old, &new),
        update_props(old.node_id(), change, Patch::NoChange)
    );
    let content = text("panel");
    let old = text("a").with_panel(PanelSpec::new(Box::new(content.clone())));
    let new = text("a").with_panel(PanelSpec::new(Box::new(text("panel2"))));
    let change = PropsChange {
        panel_patch: Some(Box::new(update_text(&content, "panel2"))),
        ..PropsChange::default()
    };
    assert_eq!(
        diff(&old, &new),
        update_props(old.node_id(), change, Patch::NoChange)
    );
}
