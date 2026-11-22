//! `ui.text`, `ui.progress`, `ui.rect` and `ui.image` in their positional and table forms.
use super::support::{class_names, eval_node, text_of};
use crate::features::vdom::domain::{NodeTag, VNodeKind};
use crate::shared::primitives::geometry::Size;

#[test]
fn test_lua_text_of_a_scalar_takes_its_text_and_class() {
    for (script, text, classes) in [
        (r#"return ui.text("hi", "a b")"#, "hi", vec!["a", "b"]),
        (r#"return ui.text(42, "n")"#, "42", vec!["n"]),
        (r#"return ui.text(1.5, "f")"#, "1.5", vec!["f"]),
        (r#"return ui.text("plain")"#, "plain", vec![]),
    ] {
        let node = eval_node(script);
        assert_eq!(text_of(&node), text, "{script}");
        assert_eq!(class_names(&node), classes, "{script}");
    }
}

#[test]
fn test_lua_text_of_another_value_is_empty_and_unclassed() {
    let node = eval_node(r#"return ui.text(true, "ignored")"#);
    assert_eq!(text_of(&node), "");
    assert!(node.class_names().is_none());
}

#[test]
fn test_lua_text_table_reads_its_props() {
    let node = eval_node(r#"return ui.text({ text = 7, class = "t", on_hover = { Exec = "h" } })"#);
    assert_eq!(text_of(&node), "7");
    assert_eq!(class_names(&node), ["t"]);
    assert_eq!(
        node.on_hover(),
        Some(&crate::features::vdom::domain::UiAction::Exec("h".into()))
    );
}

#[test]
fn test_lua_progress_of_an_integer_outside_i32_is_empty() {
    let node = eval_node("return ui.progress(1 << 40)");
    let VNodeKind::Progress { value, .. } = node.kind() else {
        panic!("expected a progress node, got {:?}", node.kind());
    };
    assert!(value.value().abs() < f32::EPSILON);
}

#[test]
fn test_lua_image_reads_data_size_class_and_tooltip_but_no_click() {
    let node = eval_node(
        r#"return ui.image({ data = "ab", pixel_size = { width = 1, height = 2 }, class = "i",
                             on_click = { Exec = "x" }, tooltip = ui.text("t") })"#,
    );
    let VNodeKind::Image { data, pixel_size } = node.kind() else {
        panic!("expected an image node, got {:?}", node.kind());
    };
    assert_eq!(
        (data.as_slice(), *pixel_size),
        (&b"ab"[..], Size::new(1, 2))
    );
    assert_eq!(class_names(&node), ["i"]);
    assert_eq!(node.tooltip().map(text_of).as_deref(), Some("t"));
    assert!(node.on_click().is_none());
}

#[test]
fn test_lua_image_keeps_its_key() {
    let node = eval_node(r#"return ui.image({ data = "ab", key = "img" })"#);
    assert_eq!(
        node.key()
            .map(crate::features::vdom::domain::NodeKey::as_str),
        Some("img")
    );
}

#[test]
fn test_lua_progress_table_keeps_its_key() {
    let node = eval_node(r#"return ui.progress({ value = 0.5, key = "bar" })"#);
    assert_eq!(
        node.key()
            .map(crate::features::vdom::domain::NodeKey::as_str),
        Some("bar")
    );
}

#[test]
fn test_lua_rect_takes_a_class_a_table_or_nothing() {
    for (script, classes) in [
        (r#"return ui.rect("box")"#, vec!["box"]),
        (r#"return ui.rect({ class = "box" })"#, vec!["box"]),
        ("return ui.rect()", vec![]),
    ] {
        let node = eval_node(script);
        assert_eq!(node.tag(), NodeTag::Rect, "{script}");
        assert_eq!(class_names(&node), classes, "{script}");
    }
}
