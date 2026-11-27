//! `diff_node_kinds`: one patch per kind of content change, `NoChange` when equal, and `None`
//! for kinds it leaves to the caller (containers).
use super::{flex, text};
use crate::features::styling::domain::{Orientation, ProgressValue};
use crate::features::vdom::adapters::kind_differ::diff_node_kinds;
use crate::features::vdom::domain::{
    ElementSelectors, ImageContent, Interactions, ModuleParams, NodeId, Patch, ProgressBar,
    TextContent, VNode,
};
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::{
    BinaryData, DynamicValue, ModuleInstanceId, ModuleName, ModuleOptions,
};
use std::collections::HashMap;

/// A progress node at `value` filling along `orientation`.
fn progress(value: f32, orientation: Orientation) -> VNode {
    let bar = ProgressBar::new(ProgressValue::new(value).unwrap(), orientation);
    VNode::new_progress(bar, ElementSelectors::default(), Interactions::default())
}

/// An image node of `bytes` at `width` x 1 pixels.
fn image(bytes: &[u8], width: u32) -> VNode {
    let content = ImageContent::new(bytes.to_vec(), Size::new(width, 1));
    VNode::new_image(content, ElementSelectors::default(), None)
}

/// Module options holding one `format` entry.
fn format_options(format: &str) -> ModuleOptions {
    let mut options = HashMap::new();
    options.insert("format".to_string(), DynamicValue::from(format));
    ModuleOptions::new(options)
}

/// A module node embedding `name`, as `instance` when given, with one `format` option.
fn module(name: &str, instance: Option<&str>, format: &str) -> VNode {
    let params = ModuleParams::new(
        ModuleName::new(name),
        instance.map(ModuleInstanceId::new),
        format_options(format),
    );
    VNode::new_module(params, ElementSelectors::default(), Interactions::default())
}

/// The kind patch from `old` to `new`, reported against `node_id`.
fn kind_patch(node_id: NodeId, old: &VNode, new: &VNode) -> Option<Patch> {
    diff_node_kinds(node_id, old.kind(), new.kind())
}

#[test]
fn equal_text_is_unchanged_and_new_text_is_an_update_text() {
    let node_id = NodeId::new();
    assert_eq!(
        kind_patch(node_id, &text("a"), &text("a")),
        Some(Patch::NoChange)
    );
    assert_eq!(
        kind_patch(node_id, &text("a"), &text("b")),
        Some(Patch::UpdateText {
            node_id,
            new_text: TextContent::new("b".to_string()),
        })
    );
}

#[test]
fn progress_value_or_orientation_change_is_an_update_progress_with_the_new_pair() {
    let node_id = NodeId::new();
    let old = progress(0.5, Orientation::Horizontal);
    let expected = |value: f32, orientation| {
        Some(Patch::UpdateProgress {
            node_id,
            new_value: ProgressValue::new(value).unwrap(),
            new_orientation: orientation,
        })
    };
    assert_eq!(
        kind_patch(node_id, &old, &old.clone()),
        Some(Patch::NoChange)
    );
    assert_eq!(
        kind_patch(node_id, &old, &progress(0.7, Orientation::Horizontal)),
        expected(0.7, Orientation::Horizontal)
    );
    assert_eq!(
        kind_patch(node_id, &old, &progress(0.5, Orientation::Vertical)),
        expected(0.5, Orientation::Vertical)
    );
}

#[test]
fn image_data_or_size_change_is_an_update_image_with_the_new_pair() {
    let node_id = NodeId::new();
    let old = image(&[1, 2], 1);
    let expected = |bytes: &[u8], width| {
        Some(Patch::UpdateImage {
            node_id,
            new_data: BinaryData::from(bytes),
            new_pixel_size: Size::new(width, 1),
        })
    };
    assert_eq!(
        kind_patch(node_id, &old, &image(&[1, 2], 1)),
        Some(Patch::NoChange)
    );
    assert_eq!(
        kind_patch(node_id, &old, &image(&[3, 4], 1)),
        expected(&[3, 4], 1)
    );
    assert_eq!(
        kind_patch(node_id, &old, &image(&[1, 2], 2)),
        expected(&[1, 2], 2)
    );
}

#[test]
fn rects_are_always_unchanged() {
    let rect = || VNode::new_rect(ElementSelectors::default(), Interactions::default());
    assert_eq!(
        kind_patch(NodeId::new(), &rect(), &rect()),
        Some(Patch::NoChange)
    );
}

#[test]
fn module_name_instance_or_options_change_is_an_update_module_with_the_new_triple() {
    let node_id = NodeId::new();
    let old = module("clock", Some("one"), "%H");
    let expected = |name: &str, instance: Option<&str>, format: &str| {
        Some(Patch::UpdateModule {
            node_id,
            new_name: ModuleName::new(name),
            new_instance_id: instance.map(ModuleInstanceId::new),
            new_options: format_options(format),
        })
    };
    assert_eq!(
        kind_patch(node_id, &old, &old.clone()),
        Some(Patch::NoChange)
    );
    for (name, instance, format) in [
        ("battery", Some("one"), "%H"),
        ("clock", Some("two"), "%H"),
        ("clock", None, "%H"),
        ("clock", Some("one"), "%M"),
    ] {
        assert_eq!(
            kind_patch(node_id, &old, &module(name, instance, format)),
            expected(name, instance, format)
        );
    }
}

#[test]
fn containers_are_left_to_the_caller() {
    let node_id = NodeId::new();
    assert_eq!(
        kind_patch(node_id, &flex(vec![]), &flex(vec![text("a")])),
        None
    );
}
