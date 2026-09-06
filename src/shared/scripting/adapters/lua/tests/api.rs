use crate::shared::primitives::geometry::{Scale, Size};
use crate::shared::primitives::{MonitorId, ScriptMonitorInfo};
use crate::shared::scripting::adapters::lua::{register_cranky_api, LuaMonitor};
use mlua::Lua;

#[test]
fn test_lua_cranky_namespaces_and_aliases() {
    let lua = Lua::new();
    register_cranky_api(&lua).expect("Registration failed");

    let script = r#"
        assert(type(cranky) == "table", "cranky must be table")
        assert(type(cranky.ui) == "table", "cranky.ui must be table")
        assert(type(cranky.ui.element) == "table", "cranky.ui.element must be table")
        assert(type(cranky.ui.action) == "table", "cranky.ui.action must be table")
        assert(type(cranky.signals) == "table", "cranky.signals must be table")
        assert(type(cranky.config) == "table", "cranky.config must be table")
        assert(type(cranky.sys) == "table", "cranky.sys must be table")

        -- Direct aliases
        assert(ui == cranky.ui, "ui must alias cranky.ui")
        assert(signals == cranky.signals, "signals must alias cranky.signals")
        assert(config == cranky.config, "config must alias cranky.config")
        assert(sys == cranky.sys, "sys must alias cranky.sys")

        -- Direct element aliases on ui
        assert(type(ui.flex) == "function", "ui.flex must be function")
        assert(type(ui.text) == "function", "ui.text must be function")
        assert(type(ui.grid) == "function", "ui.grid must be function")
        assert(type(ui.progress) == "function", "ui.progress must be function")
        assert(type(ui.rect) == "function", "ui.rect must be function")
        assert(type(ui.image) == "function", "ui.image must be function")
        assert(type(ui.module) == "function", "ui.module must be function")
        return true
    "#;
    let result = lua
        .load(script)
        .eval::<bool>()
        .expect("Script evaluation failed");
    assert!(result);
}

#[test]
fn test_lua_ui_actions() {
    let lua = Lua::new();
    register_cranky_api(&lua).expect("Registration failed");

    let script = r#"
        local a1 = ui.action.exec("echo 123")
        local a2 = ui.action.call("toggle_popup")
        local a3 = ui.action.systray(5, "Primary")
        return { a1 = a1, a2 = a2, a3 = a3 }
    "#;
    let table = lua.load(script).eval::<mlua::Table>().unwrap();
    let a1: mlua::Table = table.get("a1").unwrap();
    assert_eq!(a1.get::<String>("Exec").unwrap(), "echo 123");

    let a2: mlua::Table = table.get("a2").unwrap();
    assert_eq!(a2.get::<String>("ScriptCall").unwrap(), "toggle_popup");

    let a3: mlua::Table = table.get("a3").unwrap();
    let sub: mlua::Table = a3.get("SystrayAction").unwrap();
    assert_eq!(sub.get::<u32>("id").unwrap(), 5);
    assert_eq!(sub.get::<String>("action").unwrap(), "Primary");
}

#[test]
fn test_lua_rich_monitor_userdata() {
    let lua = Lua::new();
    let info = ScriptMonitorInfo::new(
        MonitorId::new("DP-1"),
        "DP-1".to_string(),
        Size::new(1920, 1080),
        Scale::new(1.5),
        true,
        Some(1),
        Some(2),
    );
    let mon = LuaMonitor(info);
    lua.globals().set("test_mon", mon).unwrap();

    let script = r#"
        assert(test_mon.id == "DP-1", "id field")
        assert(test_mon.name == "DP-1", "name field")
        assert(test_mon.width == 1920, "width field")
        assert(test_mon.height == 1080, "height field")
        assert(test_mon.scale == 1.5, "scale field")
        assert(test_mon.is_focused == true, "is_focused field")
        assert(test_mon.active_workspace_id == 1, "active_workspace_id field")
        assert(test_mon.special_workspace_id == 2, "special_workspace_id field")
        assert(tostring(test_mon) == "DP-1", "tostring()")
        return true
    "#;
    let result = lua.load(script).eval::<bool>().unwrap();
    assert!(result);
}
