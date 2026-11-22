//! The elements a Rhai script builds with `ui.text`, `ui.progress`, `ui.rect`, `ui.image`,
//! `ui.module`, `ui.popup` and `ui.panel`, in each argument form.
use super::support::{class_names, render_body, text_of};
use crate::features::styling::domain::Orientation;
use crate::features::vdom::domain::{AnchorDirection, NodeTag, PanelLayer, VNodeKind};
use crate::shared::primitives::{DynamicValue, ModuleInstanceId, ModuleName, ModuleOptions};

#[test]
fn test_rhai_text_takes_each_scalar_with_or_without_a_class() {
    for (expression, text, classes) in [
        (r#"ui.text(#{ text: "p", class: "c" })"#, "p", vec!["c"]),
        (r#"ui.text("s")"#, "s", vec![]),
        (r#"ui.text("s", "c")"#, "s", vec!["c"]),
        ("ui.text(42)", "42", vec![]),
        (r#"ui.text(42, "n")"#, "42", vec!["n"]),
        ("ui.text(1.5)", "1.5", vec![]),
        (r#"ui.text(1.5, "f")"#, "1.5", vec!["f"]),
    ] {
        let node = render_body(expression);
        assert_eq!(text_of(&node), text, "{expression}");
        assert_eq!(class_names(&node), classes, "{expression}");
    }
}

/// The value, orientation and class names of a progress node.
type Progress = (f32, Orientation, Vec<String>);

/// The value, orientation and classes of the progress node `expression` builds.
fn progress_of(expression: &str) -> Progress {
    let node = render_body(expression);
    let VNodeKind::Progress { value, orientation } = node.kind() else {
        panic!(
            "expected a progress node for {expression}, got {:?}",
            node.kind()
        );
    };
    (value.value(), *orientation, class_names(&node))
}

/// The progress `value` with `orientation` and the class names `class`.
fn progress(value: f32, orientation: Orientation, class: &[&str]) -> Progress {
    (
        value,
        orientation,
        class.iter().map(ToString::to_string).collect(),
    )
}

#[test]
fn test_rhai_progress_takes_props_or_a_float_value() {
    for (expression, expected) in [
        (
            r#"ui.progress(#{ value: 0.5, orientation: "vertical" })"#,
            progress(0.5, Orientation::Vertical, &[]),
        ),
        (
            "ui.progress(0.25)",
            progress(0.25, Orientation::Horizontal, &[]),
        ),
        (
            r#"ui.progress(0.25, "c")"#,
            progress(0.25, Orientation::Horizontal, &["c"]),
        ),
        (
            r#"ui.progress(0.25, "vertical", "c")"#,
            progress(0.25, Orientation::Vertical, &["c"]),
        ),
    ] {
        assert_eq!(progress_of(expression), expected, "{expression}");
    }
}

#[test]
fn test_rhai_progress_takes_an_integer_value() {
    for (expression, expected) in [
        (
            "ui.progress(1)",
            progress(1.0, Orientation::Horizontal, &[]),
        ),
        (
            r#"ui.progress(1, "c")"#,
            progress(1.0, Orientation::Horizontal, &["c"]),
        ),
        (
            r#"ui.progress(1, "vertical", "c")"#,
            progress(1.0, Orientation::Vertical, &["c"]),
        ),
    ] {
        assert_eq!(progress_of(expression), expected, "{expression}");
    }
}

#[test]
fn test_rhai_progress_of_an_integer_outside_i32_is_empty() {
    for expression in [
        "ui.progress(1 << 40)",
        r#"ui.progress(1 << 40, "c")"#,
        r#"ui.progress(1 << 40, "vertical", "c")"#,
    ] {
        assert!(
            progress_of(expression).0.abs() < f32::EPSILON,
            "{expression}"
        );
    }
}

#[test]
fn test_rhai_every_module_alias_takes_props_a_name_or_a_name_and_class() {
    let expected = VNodeKind::Module {
        name: ModuleName::new("clock"),
        instance_id: None,
        options: ModuleOptions::default(),
    };
    // `module` is a reserved word in Rhai, so scripts reach that alias only through the others.
    for alias in ["widget", "load_module", "mod_element"] {
        for (arguments, classes) in [
            (r#"#{ name: "clock", class: "m" }"#, vec!["m"]),
            (r#""clock""#, vec![]),
            (r#""clock", "m""#, vec!["m"]),
        ] {
            let node = render_body(&format!("ui.{alias}({arguments})"));
            assert_eq!(node.kind(), &expected, "{alias}({arguments})");
            assert_eq!(class_names(&node), classes, "{alias}({arguments})");
        }
    }
}

#[test]
fn test_rhai_module_props_keep_instance_and_options() {
    let node =
        render_body(r#"ui.widget(#{ name: "clock", instance_id: "c1", options: #{ fmt: "x" } })"#);
    let options = std::collections::HashMap::from([(
        "fmt".to_string(),
        DynamicValue::String("x".to_string()),
    )]);
    let expected = VNodeKind::Module {
        name: ModuleName::new("clock"),
        instance_id: Some(ModuleInstanceId::new("c1")),
        options: ModuleOptions::new(options),
    };
    assert_eq!(node.kind(), &expected);
}

#[test]
fn test_rhai_popup_and_panel_with_or_without_the_ui_target() {
    for prefix in ["ui.", ""] {
        let node = render_body(&format!(
            r#"ui.rect(#{{
                popup: {prefix}popup(#{{ content: ui.text("p"), anchor: "left" }}),
                panel: {prefix}panel(#{{ content: ui.text("q"), layer: "bottom" }}),
            }})"#
        ));
        let popup = node.popup().expect("popup expected");
        assert_eq!(text_of(popup.content()), "p", "{prefix}");
        assert_eq!(popup.anchor_direction(), AnchorDirection::Left, "{prefix}");
        let panel = node.panel().expect("panel expected");
        assert_eq!(text_of(panel.content()), "q", "{prefix}");
        assert_eq!(panel.layer(), PanelLayer::Bottom, "{prefix}");
    }
}

#[test]
fn test_rhai_popup_and_panel_are_typed_with_or_without_the_ui_target() {
    for prefix in ["ui.", ""] {
        let node = render_body(&format!(
            r#"ui.text({prefix}popup(#{{}}).type + "|" + {prefix}panel(#{{}}).type)"#
        ));
        assert_eq!(text_of(&node), "popup|panel", "{prefix}");
    }
}

#[test]
fn test_rhai_rect_and_image_are_typed_from_a_class_nothing_or_props() {
    for (expression, tag, classes) in [
        (r#"ui.rect("box")"#, NodeTag::Rect, vec!["box"]),
        ("ui.rect()", NodeTag::Rect, vec![]),
        (r#"ui.rect(#{ class: "box" })"#, NodeTag::Rect, vec!["box"]),
        (
            r#"ui.image(#{ data: blob(2, 7), pixel_size: #{ width: 1, height: 2 }, class: "i" })"#,
            NodeTag::Image,
            vec!["i"],
        ),
    ] {
        let node = render_body(expression);
        assert_eq!(node.tag(), tag, "{expression}");
        assert_eq!(class_names(&node), classes, "{expression}");
    }
}
