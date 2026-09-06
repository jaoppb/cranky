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
                    popup = ui.flex({
                        class = "popup-menu",
                        children = {
                            ui.text({ text = "Item 1" })
                        }
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
    assert_eq!(popup.children().len(), 1);
    assert_eq!(popup.tag(), crate::features::vdom::domain::NodeTag::Flex);
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
