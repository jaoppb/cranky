//! The click handlers of a Lua element and the actions `ui.action` builds.
use super::support::{api_state, eval_node};
use crate::features::systray::domain::{SystrayActionName, SystrayId};
use crate::features::vdom::domain::{ClickHandlers, UiAction};
use crate::shared::events::core::PointerButton;
use crate::shared::primitives::FunctionName;

/// The click handlers of a text node whose `on_click` is the Lua table `on_click`.
fn handlers_of(on_click: &str) -> Option<ClickHandlers> {
    let node = eval_node(&format!(
        r#"return ui.text({{ text = "t", on_click = {on_click} }})"#
    ));
    node.on_click().cloned()
}

fn exec(command: &str) -> UiAction {
    UiAction::Exec(command.to_string())
}

#[test]
fn test_lua_click_handler_keys_name_or_number_a_button() {
    for (key, button) in [
        (r#"["MIDDLE"]"#, PointerButton::Middle),
        ("[273]", PointerButton::Right),
        ("[274.0]", PointerButton::Middle),
        ("[0.0]", PointerButton::Other(0)),
        ("[4294967295.0]", PointerButton::Other(u32::MAX)),
    ] {
        let handlers = handlers_of(&format!(r#"{{ {key} = {{ Exec = "b" }} }}"#));
        let handlers = handlers.expect("handlers expected");
        assert_eq!(handlers.get(button), Some(&exec("b")), "{key}");
    }
}

#[test]
fn test_lua_click_handler_keys_that_name_no_button_are_skipped() {
    for key in [
        r#"["nope"]"#,
        "[-1]",
        "[-1.0]",
        "[273.5]",
        "[4294967296.0]",
        "[4294967296]",
        "[true]",
    ] {
        let handlers = handlers_of(&format!(
            r#"{{ {key} = {{ Exec = "skip" }}, left = {{ Exec = "l" }} }}"#
        ));
        let handlers = handlers.expect("handlers expected");
        assert_eq!(handlers, single_left(), "{key}");
    }
}

fn single_left() -> ClickHandlers {
    let mut handlers = ClickHandlers::new();
    handlers.insert(PointerButton::Left, exec("l"));
    handlers
}

#[test]
fn test_lua_click_handler_skips_values_that_are_no_action() {
    let handlers = handlers_of(r#"{ left = { Exec = "l" }, right = { Nope = 1 }, middle = 5 }"#);
    assert_eq!(handlers, Some(single_left()));
}

#[test]
fn test_lua_click_handler_without_buttons_is_one_action_or_none() {
    assert_eq!(
        handlers_of(r#"{ Exec = "all" }"#),
        Some(ClickHandlers::from_single(exec("all")))
    );
    assert_eq!(handlers_of("{ Nope = 1 }"), None);
    assert_eq!(handlers_of(r#""left""#), None);
}

/// The action the Lua expression `expression` evaluates to, read back as a `UiAction`.
fn action_of(expression: &str) -> UiAction {
    let node = eval_node(&format!(
        r#"return ui.text({{ text = "t", on_hover = {expression} }})"#
    ));
    node.on_hover().expect("action expected").clone()
}

fn systray(id: &str, action: &str) -> UiAction {
    UiAction::SystrayAction {
        id: SystrayId::new(id),
        action: SystrayActionName::parse_str(action),
        pos: None,
    }
}

#[test]
fn test_lua_action_exec_and_every_call_alias() {
    assert_eq!(action_of(r#"ui.action.exec("ls")"#), exec("ls"));
    for alias in ["call", "script_call", "call_script", "func"] {
        assert_eq!(
            action_of(&format!(r#"ui.action.{alias}("go")"#)),
            UiAction::ScriptCall(FunctionName::new("go")),
            "{alias}"
        );
    }
}

#[test]
fn test_lua_action_systray_takes_any_id_and_defaults_to_primary() {
    for (expression, expected) in [
        (
            r#"ui.action.systray("app", "Secondary")"#,
            systray("app", "Secondary"),
        ),
        ("ui.action.systray(7)", systray("7", "Primary")),
        (
            r#"ui.action.systray(2.5, "Activate")"#,
            systray("2.5", "Activate"),
        ),
        ("ui.action.systray(true)", systray("", "Primary")),
    ] {
        assert_eq!(action_of(expression), expected, "{expression}");
    }
}

#[test]
fn test_lua_action_tables_hold_the_raw_fields() {
    let lua = api_state();
    let table: mlua::Table = lua
        .load(r#"return ui.action.systray("app")"#)
        .eval()
        .unwrap();
    let inner: mlua::Table = table.get("SystrayAction").unwrap();
    assert_eq!(inner.get::<String>("id").unwrap(), "app");
    assert_eq!(inner.get::<String>("action").unwrap(), "Primary");
}
