//! The steps of a Lua module's render: calling the script with its monitor, and the empty node
//! when that fails.
use crate::features::vdom::domain::{ElementSelectors, Interactions, VNode};
use crate::shared::primitives::{MonitorId, ScriptMonitorInfo};
use crate::shared::scripting::adapters::lua::userdata::LuaMonitor;
use crate::shared::scripting::adapters::lua::vnode_parser::value_to_vnode;
use mlua::Lua;

/// Calls the script's `render` with the monitor whose id is `monitor` and converts what it
/// returns: the outer error is a missing `render`, the middle one a failed call and the inner one
/// a value that is no node.
pub(super) fn call_render(
    lua: &Lua,
    monitor: &MonitorId,
) -> mlua::Result<mlua::Result<mlua::Result<VNode>>> {
    let globals = lua.globals();
    let lua_monitor = monitor_for_render(&globals, monitor);
    globals.get::<mlua::Function>("render").map(|render_fn| {
        render_fn
            .call::<mlua::Value>(lua_monitor)
            .map(|val| value_to_vnode(lua, val))
    })
}

/// The monitor the last refresh listed with the id `monitor`, or one built from the id when it
/// listed none.
fn monitor_for_render(globals: &mlua::Table, monitor: &MonitorId) -> LuaMonitor {
    let refreshed = globals
        .get::<Option<mlua::Table>>("_raw_monitors")
        .ok()
        .flatten()
        .and_then(|raw_monitors| find_raw_monitor(&raw_monitors, monitor));
    LuaMonitor(refreshed.unwrap_or_else(|| ScriptMonitorInfo::from_id(monitor)))
}

/// The first monitor of `raw_monitors` whose id is `monitor`, skipping entries that are no
/// monitor.
fn find_raw_monitor(raw_monitors: &mlua::Table, monitor: &MonitorId) -> Option<ScriptMonitorInfo> {
    raw_monitors
        .sequence_values::<mlua::Value>()
        .find_map(|pair| {
            let Ok(mlua::Value::UserData(user_data)) = pair else {
                return None;
            };
            let mon = user_data.borrow::<LuaMonitor>().ok()?;
            (mon.0.id() == monitor).then(|| mon.0.clone())
        })
}

/// The node a module renders when its script produced none: an empty flex.
pub(super) fn empty_flex() -> VNode {
    VNode::new_flex(vec![], ElementSelectors::default(), Interactions::default())
}
