//! The `sys` table a Lua script sees: commands, environment, host name, logging and monitors.
use super::support::api_state;
use crate::shared::primitives::{MonitorFocus, MonitorId, ScriptMonitorInfo};
use crate::shared::scripting::adapters::lua::LuaMonitor;
use crate::shared::scripting::adapters::test_support::{file_appears, marker_path};

#[test]
fn test_lua_sys_exec_runs_the_command_in_a_shell() {
    let marker = marker_path("lua-sys-exec");
    let lua = api_state();
    lua.load(format!(r#"sys.exec("echo ok > '{}'")"#, marker.display()))
        .exec()
        .unwrap();
    assert!(file_appears(&marker), "the command never ran");
}

#[test]
fn test_lua_sys_env_reads_a_variable_or_nil() {
    let lua = api_state();
    let home: Option<String> = lua.load(r#"return sys.env("PATH")"#).eval().unwrap();
    assert_eq!(home, std::env::var("PATH").ok());
    let missing: Option<String> = lua
        .load(r#"return sys.env("CRANKY_TEST_SURELY_UNSET_VARIABLE")"#)
        .eval()
        .unwrap();
    assert_eq!(missing, None);
}

#[test]
fn test_lua_sys_hostname_reads_etc_hostname_first() {
    let lua = api_state();
    let name: String = lua.load("return sys.hostname()").eval().unwrap();
    let expected = std::fs::read_to_string("/etc/hostname").map_or_else(
        |_| std::env::var("HOSTNAME").unwrap_or_else(|_| "localhost".to_string()),
        |contents| contents.trim().to_string(),
    );
    assert_eq!(name, expected);
}

#[test]
fn test_lua_sys_log_methods_accept_a_message() {
    let lua = api_state();
    lua.load(r#"sys.log.debug("d") sys.log.info("i") sys.log.warn("w") sys.log.error("e")"#)
        .exec()
        .unwrap();
}

/// A Lua state whose raw monitors are DP-1 and the focused DP-2.
fn state_with_monitors() -> mlua::Lua {
    let lua = api_state();
    let first = LuaMonitor(ScriptMonitorInfo::from_id(&MonitorId::new("DP-1")));
    let focused =
        ScriptMonitorInfo::from_id(&MonitorId::new("DP-2")).with_focused(MonitorFocus::Focused);
    let monitors = lua
        .create_sequence_from([first, LuaMonitor(focused.clone())])
        .unwrap();
    lua.globals().set("_raw_monitors", monitors).unwrap();
    lua.globals()
        .set("_raw_focused_monitor", LuaMonitor(focused))
        .unwrap();
    lua
}

#[test]
fn test_lua_sys_monitors_list_find_and_focus() {
    let lua = state_with_monitors();
    let script = r#"
        local ids = {}
        for _, monitor in ipairs(sys.monitors.all()) do ids[#ids + 1] = monitor.id end
        return table.concat(ids, ",") .. "|" .. sys.monitors.focused().id .. "|"
            .. sys.monitors.current().id .. "|" .. sys.monitors.get("DP-1").id .. "|"
            .. tostring(sys.monitors.get("HDMI-A-1"))
    "#;
    let summary: String = lua.load(script).eval().unwrap();
    assert_eq!(summary, "DP-1,DP-2|DP-2|DP-2|DP-1|nil");
}

#[test]
fn test_lua_sys_monitors_without_raw_monitors_are_empty() {
    let lua = api_state();
    let script = r#"
        return #sys.monitors.all() .. "|" .. tostring(sys.monitors.focused()) .. "|"
            .. tostring(sys.monitors.current()) .. "|" .. tostring(sys.monitors.get("DP-1"))
    "#;
    let summary: String = lua.load(script).eval().unwrap();
    assert_eq!(summary, "0|nil|nil|nil");
}

#[test]
fn test_lua_sys_monitors_get_skips_entries_that_are_no_monitor() {
    let lua = state_with_monitors();
    lua.load(r#"table.insert(_raw_monitors, 1, "DP-1")"#)
        .exec()
        .unwrap();
    let id: String = lua
        .load(r#"return sys.monitors.get("DP-1").id"#)
        .eval()
        .unwrap();
    assert_eq!(id, "DP-1");
}
