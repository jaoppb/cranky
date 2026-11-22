//! The popup and panel specifications a Lua element carries.
use super::support::{eval_node, eval_node_error, text_of};
use crate::features::vdom::domain::{
    AnchorDirection, ExclusiveZone, KeyboardInteractivity, PanelLayer, PanelSpec, PopupOffset,
    PopupSpec, VNode,
};
use crate::shared::config::domain::{AxisMargins, MarginConfig, MarginOffset};

/// The node `ui.text` returns with the extra props `props`.
fn text_with(props: &str) -> VNode {
    eval_node(&format!(r#"return ui.text({{ text = "t", {props} }})"#))
}

/// The popup a text node with the popup table fields `fields` opens.
fn popup_of(fields: &str) -> PopupSpec {
    let node = text_with(&format!(
        r#"popup = {{ content = ui.text("c"), {fields} }}"#
    ));
    node.popup().expect("popup expected").clone()
}

/// The panel a text node with the panel table fields `fields` opens.
fn panel_of(fields: &str) -> PanelSpec {
    let node = text_with(&format!(
        r#"panel = {{ content = ui.text("c"), {fields} }}"#
    ));
    node.panel().expect("panel expected").clone()
}

#[test]
fn test_lua_popup_anchor_names_each_side_and_defaults_to_auto() {
    for (fields, anchor) in [
        (r#"anchor = "top""#, AnchorDirection::Top),
        (r#"anchor = "bottom""#, AnchorDirection::Bottom),
        (r#"anchor = "left""#, AnchorDirection::Left),
        (r#"anchor = "right""#, AnchorDirection::Right),
        (r#"anchor = "Top""#, AnchorDirection::Auto),
        ("", AnchorDirection::Auto),
    ] {
        let popup = popup_of(fields);
        assert_eq!(popup.anchor_direction(), anchor, "{fields}");
        assert_eq!(text_of(popup.content()), "c");
    }
}

#[test]
fn test_lua_popup_offset_reads_x_y_or_dx_dy() {
    for (fields, offset) in [
        ("offset = { x = 1, y = 2 }", Some(PopupOffset::new(1, 2))),
        ("offset = { dx = 3, dy = 4 }", Some(PopupOffset::new(3, 4))),
        ("offset = { x = 5, dy = 6 }", Some(PopupOffset::new(5, 6))),
        ("offset = {}", Some(PopupOffset::new(0, 0))),
        ("", None),
    ] {
        assert_eq!(popup_of(fields).offset(), offset, "{fields}");
    }
}

#[test]
fn test_lua_popup_of_a_node_wraps_it() {
    let popup = text_with(r#"popup = ui.text("inline")"#).popup().cloned();
    let popup = popup.expect("popup expected");
    assert_eq!(text_of(popup.content()), "inline");
    assert_eq!(popup.anchor_direction(), AnchorDirection::Auto);
    assert_eq!(popup.offset(), None);
}

#[test]
fn test_lua_popup_or_panel_of_another_value_is_none() {
    for props in [
        r#"popup = "x""#,
        r#"popup = ui.panel({ content = ui.text("p") })"#,
        r#"panel = "x""#,
        r#"panel = ui.text("p")"#,
    ] {
        let node = text_with(props);
        assert!(node.popup().is_none() && node.panel().is_none(), "{props}");
    }
}

#[test]
fn test_lua_popup_or_panel_table_without_content_is_an_error() {
    // `node` is read only when reading `content` fails, which a plain table never does, so a
    // table without `content` converts nil and fails.
    for props in [
        r#"popup = { node = ui.text("n") }"#,
        r#"panel = { node = ui.text("n") }"#,
    ] {
        eval_node_error(&format!(r#"return ui.text({{ text = "t", {props} }})"#));
    }
}

#[test]
fn test_lua_popup_and_panel_constructors_reject_what_they_cannot_read() {
    let popup_error = eval_node_error(r#"return ui.popup("x")"#);
    assert!(
        popup_error.contains("Invalid popup specification"),
        "{popup_error}"
    );
    let panel_error = eval_node_error(r#"return ui.panel(ui.text("x"))"#);
    assert!(
        panel_error.contains("Invalid panel specification"),
        "{panel_error}"
    );
}

#[test]
fn test_lua_panel_layer_names_each_layer_and_defaults_to_top() {
    for (fields, layer) in [
        (r#"layer = "background""#, PanelLayer::Background),
        (r#"layer = "bottom""#, PanelLayer::Bottom),
        (r#"layer = "overlay""#, PanelLayer::Overlay),
        (r#"layer = "top""#, PanelLayer::Top),
        (r#"layer = "Overlay""#, PanelLayer::Top),
        ("", PanelLayer::Top),
    ] {
        assert_eq!(panel_of(fields).layer(), layer, "{fields}");
    }
}

#[test]
fn test_lua_panel_keyboard_names_each_mode_and_defaults_to_none() {
    for (fields, keyboard) in [
        (
            r#"keyboard = "exclusive""#,
            KeyboardInteractivity::Exclusive,
        ),
        (r#"keyboard = "on_demand""#, KeyboardInteractivity::OnDemand),
        (r#"keyboard = "none""#, KeyboardInteractivity::None),
        (r#"keyboard = "Exclusive""#, KeyboardInteractivity::None),
        ("", KeyboardInteractivity::None),
    ] {
        assert_eq!(panel_of(fields).keyboard(), keyboard, "{fields}");
    }
}

#[test]
fn test_lua_panel_reads_its_exclusive_zone_and_content() {
    let panel = panel_of("exclusive_zone = 5");
    assert_eq!(panel.exclusive_zone(), ExclusiveZone::new(5));
    assert_eq!(text_of(panel.content()), "c");
    assert_eq!(panel_of("").exclusive_zone(), ExclusiveZone::new(0));
}

/// A margin with the given top, right, bottom and left offsets.
fn margin(sides: [i32; 4]) -> MarginConfig {
    let [top, right, bottom, left] = sides;
    MarginConfig::new(
        AxisMargins::new(MarginOffset::new(top), MarginOffset::new(bottom)),
        AxisMargins::new(MarginOffset::new(left), MarginOffset::new(right)),
    )
}

#[test]
fn test_lua_panel_margin_reads_each_side_and_defaults_to_zero() {
    for (fields, expected) in [
        (
            "margin = { top = 1, right = 2, bottom = 3, left = 4 }",
            margin([1, 2, 3, 4]),
        ),
        ("margin = { bottom = 7 }", margin([0, 0, 7, 0])),
        ("margin = { left = 9 }", margin([0, 0, 0, 9])),
        ("", MarginConfig::default()),
    ] {
        assert_eq!(panel_of(fields).margin(), &expected, "{fields}");
    }
}
