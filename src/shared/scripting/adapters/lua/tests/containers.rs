//! `ui.flex`, `ui.grid` and `ui.module` in their full, shorthand and fallback forms.
use super::support::{class_names, eval_node, eval_node_error, text_of};
use crate::features::vdom::domain::{NodeKey, NodeTag, VNodeKind};
use crate::shared::primitives::{DynamicValue, ModuleInstanceId, ModuleName, ModuleOptions};

/// Each key that makes a container table a full specification, with a value it accepts.
const FULL_SPEC_KEYS: [(&str, &str); 10] = [
    ("type", r#""flex""#),
    ("class", r#""c""#),
    ("children", "{}"),
    ("id", r#""i""#),
    ("key", r#""k""#),
    ("on_click", r#"{ Exec = "x" }"#),
    ("on_hover", r#"{ Exec = "x" }"#),
    ("tooltip", r#"ui.text("t")"#),
    ("popup", r#"ui.popup({ content = ui.text("p") })"#),
    ("panel", r#"ui.panel({ content = ui.text("p") })"#),
];

#[test]
fn test_lua_container_with_any_full_spec_key_reads_children_from_the_children_key() {
    for constructor in ["flex", "grid"] {
        for (key, value) in FULL_SPEC_KEYS {
            let node = eval_node(&format!(
                r#"return ui.{constructor}({{ {key} = {value}, ui.text("ignored") }})"#
            ));
            assert!(node.children().is_empty(), "{constructor} {key}");
        }
    }
}

#[test]
fn test_lua_container_shorthand_takes_its_sequence_as_children() {
    for (constructor, tag) in [("flex", NodeTag::Flex), ("grid", NodeTag::Grid)] {
        let node = eval_node(&format!(
            r#"return ui.{constructor}({{ ui.text("a"), {{ type = "text", text = "b" }} }})"#
        ));
        assert_eq!(node.tag(), tag);
        let texts: Vec<String> = node.children().iter().map(text_of).collect();
        assert_eq!(texts, ["a", "b"], "{constructor}");
        assert_eq!(class_names(&node), Vec::<String>::new(), "{constructor}");
    }
}

#[test]
fn test_lua_container_of_a_non_table_is_empty() {
    for (constructor, tag) in [("flex", NodeTag::Flex), ("grid", NodeTag::Grid)] {
        let node = eval_node(&format!(r#"return ui.{constructor}("x")"#));
        assert_eq!(node.tag(), tag);
        assert!(node.children().is_empty() && node.class_names().is_none());
    }
}

#[test]
fn test_lua_container_shorthand_with_a_bad_child_is_an_error() {
    for constructor in ["flex", "grid"] {
        eval_node_error(&format!(
            r#"return ui.{constructor}({{ {{ type = "x" }} }})"#
        ));
    }
}

/// A module kind named `name`, with `instance_id` and the string `options`.
fn module_kind(name: &str, instance_id: Option<&str>, options: &[(&str, &str)]) -> VNodeKind {
    let map = options
        .iter()
        .map(|(key, value)| {
            (
                (*key).to_string(),
                DynamicValue::String((*value).to_string()),
            )
        })
        .collect();
    VNodeKind::Module {
        name: ModuleName::new(name),
        instance_id: instance_id.map(ModuleInstanceId::new),
        options: ModuleOptions::new(map),
    }
}

#[test]
fn test_lua_module_table_reads_name_instance_options_and_class() {
    let node = eval_node(
        r#"return ui.module({ name = "clock", instance_id = "c2", options = { fmt = "x" }, class = "m" })"#,
    );
    assert_eq!(
        node.kind(),
        &module_kind("clock", Some("c2"), &[("fmt", "x")])
    );
    assert_eq!(class_names(&node), ["m"]);
    let bare = eval_node(r#"return ui.widget({ name = "clock" })"#);
    assert_eq!(bare.kind(), &module_kind("clock", None, &[]));
}

#[test]
fn test_lua_module_of_a_name_or_another_value() {
    let named = eval_node(r#"return ui.load_module("clock")"#);
    assert_eq!(named.kind(), &module_kind("clock", None, &[]));
    assert!(named.class_names().is_none());
    let other = eval_node("return ui.module(5)");
    assert_eq!(other.kind(), &module_kind("", None, &[]));
}

#[test]
fn test_lua_module_table_without_a_name_is_an_error() {
    eval_node_error(r#"return ui.module({ class = "m" })"#);
}

#[test]
fn test_lua_container_or_module_with_a_wrongly_typed_field_is_an_error() {
    for script in [
        "return ui.flex({ class = {} })",
        r#"return ui.grid({ children = { { type = "x" } } })"#,
        r#"return ui.module({ name = "m", instance_id = {} })"#,
        r#"return ui.module({ name = "m", class = {} })"#,
    ] {
        eval_node_error(script);
    }
}

#[test]
fn test_lua_grid_and_module_tables_keep_their_key() {
    for script in [
        r#"return ui.grid({ key = "k", children = {} })"#,
        r#"return ui.module({ name = "clock", key = "k" })"#,
    ] {
        let node = eval_node(script);
        assert_eq!(node.key().map(NodeKey::as_str), Some("k"), "{script}");
    }
}
