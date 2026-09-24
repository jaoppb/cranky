#[cfg(test)]
mod tests {
    use crate::features::vdom::domain::UiAction;
    use crate::shared::events::core::PointerButton;
    use crate::shared::scripting::adapters::lua::{register_cranky_api, value_to_vnode};
    use mlua::Lua;

#[test]
fn test_vdom_dsl_constructors() {
    let lua = Lua::new();
    register_cranky_api(&lua).expect("DSL registration failed");

    let script = r#"
        local txt = ui.text({ text = "hello", class = "greeting" })
        local rect = ui.rect({ class = "box" })
        local prog = ui.progress({ value = 0.75, orientation = "vertical" })
        local flex = ui.flex({
            class = "root",
            children = { txt, rect, prog }
        })
        return flex
    "#;
    let val = lua.load(script).eval::<mlua::Value>().expect("Eval failed");
    let vnode = value_to_vnode(&lua, val).expect("Conversion failed");
    assert_eq!(vnode.tag(), crate::features::vdom::domain::NodeTag::Flex);
    assert_eq!(vnode.children().len(), 3);
    assert_eq!(
        vnode.children()[0].tag(),
        crate::features::vdom::domain::NodeTag::Text
    );
    assert_eq!(
        vnode.children()[1].tag(),
        crate::features::vdom::domain::NodeTag::Rect
    );
    assert_eq!(
        vnode.children()[2].tag(),
        crate::features::vdom::domain::NodeTag::Progress
    );
}

#[test]
fn test_lua_vnode_click_handlers() {
    let lua = Lua::new();
    register_cranky_api(&lua).expect("DSL registration failed");

    let single_script = r#"
        return ui.text({
            text = "click me",
            on_click = { Exec = "echo single" }
        })
    "#;
    let single_val = lua.load(single_script).eval::<mlua::Value>().unwrap();
    let single_node = value_to_vnode(&lua, single_val).unwrap();
    let single_handlers = single_node.on_click().expect("on_click expected");
    assert_eq!(
        single_handlers.get(&PointerButton::Left),
        Some(&UiAction::Exec("echo single".into()))
    );
    assert_eq!(
        single_handlers.get(&PointerButton::Right),
        Some(&UiAction::Exec("echo single".into()))
    );
    assert_eq!(
        single_handlers.get(&PointerButton::Middle),
        Some(&UiAction::Exec("echo single".into()))
    );

    let multi_script = r#"
        return ui.text({
            text = "multi click",
            on_click = {
                left = { Exec = "echo left" },
                right = { Exec = "echo right" },
                side = { Exec = "echo side" },
                [276] = { Exec = "echo extra" }
            }
        })
    "#;
    let multi_val = lua.load(multi_script).eval::<mlua::Value>().unwrap();
    let multi_node = value_to_vnode(&lua, multi_val).unwrap();
    let multi_handlers = multi_node.on_click().expect("on_click expected");
    assert_eq!(
        multi_handlers.get(&PointerButton::Left),
        Some(&UiAction::Exec("echo left".into()))
    );
    assert_eq!(
        multi_handlers.get(&PointerButton::Right),
        Some(&UiAction::Exec("echo right".into()))
    );
    assert_eq!(
        multi_handlers.get(&PointerButton::Side),
        Some(&UiAction::Exec("echo side".into()))
    );
    assert_eq!(
        multi_handlers.get(&PointerButton::Extra),
        Some(&UiAction::Exec("echo extra".into()))
    );
    assert_eq!(multi_handlers.get(&PointerButton::Middle), None);
}

#[test]
fn test_lua_vnode_with_popup() {
    let lua = Lua::new();
    register_cranky_api(&lua).expect("DSL registration failed");

    let script = r#"
        return ui.flex({
            children = {
                ui.text({
                    text = "Show Popup",
                    popup = ui.popup({
                        anchor = "bottom",
                        offset = { x = 0, y = 8 },
                        content = ui.flex({
                            class = "popup-menu",
                            children = {
                                ui.text({ text = "Item 1" })
                            }
                        })
                    })
                })
            }
        })
    "#;
    let val = lua.load(script).eval::<mlua::Value>().unwrap();
    let node = value_to_vnode(&lua, val).unwrap();
    assert_eq!(node.children().len(), 1);
    let button_node = &node.children()[0];
    assert!(button_node.popup().is_some());
    let popup = button_node.popup().unwrap();
    assert_eq!(popup.content().children().len(), 1);
    assert_eq!(
        popup.content().tag(),
        crate::features::vdom::domain::NodeTag::Flex
    );
    assert_eq!(
        popup.anchor_direction(),
        crate::features::vdom::domain::AnchorDirection::Bottom
    );
    assert_eq!(
        popup.offset(),
        Some(crate::features::vdom::domain::PopupOffset::new(0, 8))
    );
}

#[test]
fn test_lua_vnode_with_popup_default_offset_none() {
    let lua = Lua::new();
    register_cranky_api(&lua).expect("DSL registration failed");

    let script = r#"
        return ui.text({
            text = "Trigger",
            popup = ui.popup({
                content = ui.text({ text = "Popup Content" })
            })
        })
    "#;
    let val = lua.load(script).eval::<mlua::Value>().unwrap();
    let node = value_to_vnode(&lua, val).unwrap();
    assert!(node.popup().is_some());
    let popup = node.popup().unwrap();
    assert_eq!(popup.offset(), None);
}

#[test]
fn test_lua_vnode_with_panel() {
    let lua = Lua::new();
    register_cranky_api(&lua).expect("DSL registration failed");

    let script = r#"
        return ui.text({
            text = "Panel Trigger",
            panel = ui.panel({
                layer = "overlay",
                anchor = { "top", "right" },
                margin = { top = 10, right = 20 },
                exclusive_zone = 0,
                content = ui.flex({
                    class = "control-center",
                    children = {
                        ui.text({ text = "Volume" })
                    }
                })
            })
        })
    "#;
    let val = lua.load(script).eval::<mlua::Value>().unwrap();
    let node = value_to_vnode(&lua, val).unwrap();
    assert!(node.panel().is_some());
    let panel = node.panel().unwrap();
    assert_eq!(
        panel.layer(),
        crate::features::vdom::domain::PanelLayer::Overlay
    );
    assert!(panel.anchor().top());
    assert!(panel.anchor().right());
    assert!(!panel.anchor().bottom());
    assert_eq!(panel.margin().top().value(), 10);
    assert_eq!(panel.margin().right().value(), 20);
    assert_eq!(panel.content().children().len(), 1);
}

#[test]
fn test_lua_vnode_grid() {
    let lua = Lua::new();
    register_cranky_api(&lua).expect("DSL registration failed");

    let script = r#"
        return ui.grid({
            class = "my-grid",
            children = {
                ui.text({ text = "Cell 1" }),
                ui.text({ text = "Cell 2" })
            }
        })
    "#;
    let val = lua.load(script).eval::<mlua::Value>().unwrap();
    let node = value_to_vnode(&lua, val).unwrap();
    assert_eq!(node.tag(), crate::features::vdom::domain::NodeTag::Grid);
    assert_eq!(node.children().len(), 2);
}

#[test]
fn test_lua_shorthand_dsl() {
    let lua = Lua::new();
    register_cranky_api(&lua).expect("Registration failed");

    let script = r#"
        local t = ui.text("Hello World", "greeting-cls")
        local p = ui.progress(0.75, "vertical", "cpu-bar")
        local r = ui.rect("separator")
        local m = ui.module("clock")
        local root = ui.flex({ t, p, r, m })
        return root
    "#;
    let val = lua.load(script).eval::<mlua::Value>().unwrap();
    let vnode = value_to_vnode(&lua, val).unwrap();
    assert_eq!(vnode.tag(), crate::features::vdom::domain::NodeTag::Flex);
    assert_eq!(vnode.children().len(), 4);
    assert_eq!(
        vnode.children()[0].tag(),
        crate::features::vdom::domain::NodeTag::Text
    );
    assert_eq!(
        vnode.children()[1].tag(),
        crate::features::vdom::domain::NodeTag::Progress
    );
    assert_eq!(
        vnode.children()[2].tag(),
        crate::features::vdom::domain::NodeTag::Rect
    );
    assert_eq!(
        vnode.children()[3].tag(),
        crate::features::vdom::domain::NodeTag::Module
    );
}
#[test]
fn test_lua_vnode_key_is_reconciliation_identity() {
    let lua = Lua::new();
    register_cranky_api(&lua).expect("DSL registration failed");

    // `key` is distinct from `id`: both may be set, and either may be set
    // alone, since one is a CSS selector and the other is reconciliation
    // identity (see `NodeKey`).
    let script = r#"
        return ui.flex({
            key = "ws-3",
            id = "ws-3",
            children = {}
        })
    "#;
    let val = lua.load(script).eval::<mlua::Value>().expect("Eval failed");
    let vnode = value_to_vnode(&lua, val).expect("Conversion failed");
    assert_eq!(vnode.key().map(crate::features::vdom::domain::NodeKey::as_str), Some("ws-3"));
    assert_eq!(vnode.element_id().map(crate::features::styling::domain::ElementId::as_str), Some("ws-3"));
}

#[test]
fn test_lua_vnode_without_key_has_none() {
    let lua = Lua::new();
    register_cranky_api(&lua).expect("DSL registration failed");

    let val = lua
        .load(r#"return ui.rect("box")"#)
        .eval::<mlua::Value>()
        .expect("Eval failed");
    let vnode = value_to_vnode(&lua, val).expect("Conversion failed");
    assert_eq!(vnode.key(), None);
}
}
