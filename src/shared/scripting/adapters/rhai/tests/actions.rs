//! The actions, `sys` functions and script helpers a Rhai script calls.
use super::support::{call_error, init_module, render_body, text_of};
use crate::features::module_runtime::ports::AnyModulePort;
use crate::features::systray::domain::{SystrayActionName, SystrayId};
use crate::features::vdom::domain::UiAction;
use crate::shared::events::core::PointerButton;
use crate::shared::primitives::FunctionName;
use crate::shared::scripting::adapters::test_support::{file_appears, marker_path};

fn systray(id: &str, action: &str) -> UiAction {
    UiAction::SystrayAction {
        id: SystrayId::new(id),
        action: SystrayActionName::parse_str(action),
        pos: None,
    }
}

/// The action bound to the left button of a text node whose `on_click` is `expression`.
fn click_action_of(expression: &str) -> UiAction {
    let node = render_body(&format!(
        r#"ui.text(#{{ text: "t", on_click: {expression} }})"#
    ));
    let handlers = node.on_click().expect("handlers expected");
    handlers
        .get(PointerButton::Left)
        .expect("left action expected")
        .clone()
}

#[test]
fn test_rhai_systray_action_takes_a_string_or_integer_id_and_defaults_to_primary() {
    for (expression, expected) in [
        (
            r#"ui.action.systray("app", "Secondary")"#,
            systray("app", "Secondary"),
        ),
        (r#"ui.action.systray("app")"#, systray("app", "Primary")),
        (
            r#"ui.action.systray(7, "Activate")"#,
            systray("7", "Activate"),
        ),
        ("ui.action.systray(7)", systray("7", "Primary")),
    ] {
        assert_eq!(click_action_of(expression), expected, "{expression}");
    }
}

#[test]
fn test_rhai_every_script_call_alias_builds_a_script_call() {
    // `call` names the method Rhai calls function pointers with, so `ui.action.call` never
    // reaches the alias registered under that name.
    for alias in ["script_call", "call_script", "func"] {
        assert_eq!(
            click_action_of(&format!(r#"ui.action.{alias}("go")"#)),
            UiAction::ScriptCall(FunctionName::new("go")),
            "{alias}"
        );
    }
}

#[test]
fn test_rhai_exec_outside_ui_action_runs_the_command() {
    for call in ["sys.exec", "exec"] {
        let marker = marker_path("rhai-exec");
        let node = render_body(&format!(
            r#"let result = {call}("echo ok > '{}'"); ui.text(type_of(result))"#,
            marker.display()
        ));
        assert_eq!(text_of(&node), "()", "{call}");
        assert!(file_appears(&marker), "{call} never ran the command");
    }
}

#[test]
fn test_rhai_assert_passes_on_true_and_fails_with_its_message() {
    let mut module = init_module(
        "asserts",
        r#"fn pass() { assert(true); assert(true, "unused"); }
           fn bare() { assert(false); }
           fn message() { assert(false, "custom"); }"#,
    );
    assert!(module.call_function(&FunctionName::new("pass")).is_ok());
    let bare = call_error(&mut module, "bare", &[]);
    assert!(bare.contains("Assertion failed in Rhai script"), "{bare}");
    assert!(!bare.contains("Assertion failed in Rhai script:"), "{bare}");
    let message = call_error(&mut module, "message", &[]);
    assert!(
        message.contains("Assertion failed in Rhai script: custom"),
        "{message}"
    );
}
