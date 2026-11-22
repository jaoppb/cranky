//! The nodes a Lua script describes as plain tables with a `type`, rather than with `ui.*`.
use super::support::{class_names, eval_node, eval_node_error, text_of};
use crate::features::styling::domain::{Orientation, ProgressValue};
use crate::features::vdom::domain::{NodeTag, VNodeKind};
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::{DynamicValue, ModuleInstanceId, ModuleName, ModuleOptions};

#[test]
fn test_lua_table_flex_and_untyped_table_read_their_children_and_class() {
    for script in [
        r#"return { type = "flex", class = "row", children = { ui.text("a"), { type = "rect" } } }"#,
        r#"return { class = "row", children = { ui.text("a"), { type = "rect" } } }"#,
    ] {
        let node = eval_node(script);
        assert_eq!(node.tag(), NodeTag::Flex, "{script}");
        assert_eq!(class_names(&node), ["row"], "{script}");
        let tags: Vec<NodeTag> = node
            .children()
            .iter()
            .map(crate::features::vdom::domain::VNode::tag)
            .collect();
        assert_eq!(tags, [NodeTag::Text, NodeTag::Rect], "{script}");
    }
}

#[test]
fn test_lua_table_grid_reads_its_children() {
    let node = eval_node(r#"return { type = "grid", children = { ui.text("a"), ui.text("b") } }"#);
    assert_eq!(node.tag(), NodeTag::Grid);
    let texts: Vec<String> = node.children().iter().map(text_of).collect();
    assert_eq!(texts, ["a", "b"]);
}

#[test]
fn test_lua_table_text_reads_its_text_class_and_key() {
    let node = eval_node(r#"return { type = "text", text = "hi", class = "t", key = "k1" }"#);
    assert_eq!(text_of(&node), "hi");
    assert_eq!(class_names(&node), ["t"]);
    assert_eq!(
        node.key()
            .map(crate::features::vdom::domain::NodeKey::as_str),
        Some("k1")
    );
}

/// The value and orientation of the progress node a table describes.
fn table_progress(script: &str) -> (f32, Orientation) {
    let node = eval_node(script);
    let VNodeKind::Progress { value, orientation } = node.kind() else {
        panic!("expected a progress node, got {:?}", node.kind());
    };
    (value.value(), *orientation)
}

#[test]
fn test_lua_table_progress_reads_value_and_orientation() {
    let (value, orientation) =
        table_progress(r#"return { type = "progress", value = 0.5, orientation = "VERTICAL" }"#);
    assert!((value - 0.5).abs() < f32::EPSILON);
    assert_eq!(orientation, Orientation::Vertical);

    let (value, orientation) = table_progress(r#"return { type = "progress" }"#);
    assert!((value - ProgressValue::default().value()).abs() < f32::EPSILON);
    assert_eq!(orientation, Orientation::Horizontal);
}

#[test]
fn test_lua_table_rect_reads_its_id() {
    let node = eval_node(r#"return { type = "rect", id = "r1" }"#);
    assert_eq!(node.tag(), NodeTag::Rect);
    assert_eq!(
        node.element_id()
            .map(crate::features::styling::domain::ElementId::as_str),
        Some("r1")
    );
}

#[test]
fn test_lua_table_image_reads_data_size_and_tooltip_but_no_click() {
    let node = eval_node(
        r#"return { type = "image", data = "abc", pixel_size = { width = 2, height = 3 },
                    on_click = { Exec = "x" }, tooltip = ui.text("tip") }"#,
    );
    let VNodeKind::Image { data, pixel_size } = node.kind() else {
        panic!("expected an image node, got {:?}", node.kind());
    };
    assert_eq!(data.as_slice(), b"abc");
    assert_eq!(*pixel_size, Size::new(2, 3));
    assert_eq!(node.tooltip().map(text_of).as_deref(), Some("tip"));
    assert!(node.on_click().is_none());
}

#[test]
fn test_lua_table_module_reads_name_instance_and_options() {
    let node = eval_node(
        r#"return { type = "module", name = "clock", instance_id = "c1", options = { fmt = "x" } }"#,
    );
    let options = ModuleOptions::new(std::collections::HashMap::from([(
        "fmt".to_string(),
        DynamicValue::String("x".to_string()),
    )]));
    let expected = VNodeKind::Module {
        name: ModuleName::new("clock"),
        instance_id: Some(ModuleInstanceId::new("c1")),
        options,
    };
    assert_eq!(node.kind(), &expected);
}

#[test]
fn test_lua_table_module_without_a_name_is_an_error() {
    let error = eval_node_error(r#"return { type = "module" }"#);
    assert!(error.contains("nil to String"), "{error}");
}

#[test]
fn test_lua_table_of_another_type_is_deserialized_or_rejected() {
    let error = eval_node_error(r#"return { type = "spinner" }"#);
    assert!(error.contains("spinner"), "{error}");
}

#[test]
fn test_lua_table_with_a_wrongly_typed_field_is_an_error() {
    for script in [
        "return { type = {} }",
        r#"return { type = "progress", value = "x" }"#,
        r#"return { type = "progress", orientation = {} }"#,
        r#"return { type = "module", name = "m", instance_id = {} }"#,
        r#"return { type = "image", pixel_size = { width = "w" } }"#,
        r#"return { type = "rect", class = {} }"#,
        r#"return { type = "grid", children = { { type = "x" } } }"#,
    ] {
        eval_node_error(script);
    }
}
