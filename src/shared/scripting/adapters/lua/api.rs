use super::api_elements::{create_ui_action_table, create_ui_element_table};
use super::api_sys::create_sys_table;
use mlua::Lua;

/// Registers the `cranky`, `ui`, `signals`, `config`, and `sys` APIs in Lua globals.
///
/// # Errors
///
/// Returns `mlua::Error` if table creation or function registration fails.
pub fn register_cranky_api(lua: &Lua) -> mlua::Result<()> {
    let ui_table = lua.create_table()?;
    let element_table = create_ui_element_table(lua)?;
    let action_table = create_ui_action_table(lua)?;

    ui_table.set("element", element_table.clone())?;
    ui_table.set("action", action_table)?;
    for (k, v) in element_table.pairs::<String, mlua::Function>().flatten() {
        ui_table.set(k, v)?;
    }

    let signals_table = lua.create_table()?;
    let config_table = lua.create_table()?;
    let sys_table = create_sys_table(lua)?;

    let cranky_table = lua.create_table()?;
    cranky_table.set("ui", ui_table.clone())?;
    cranky_table.set("signals", signals_table.clone())?;
    cranky_table.set("config", config_table.clone())?;
    cranky_table.set("sys", sys_table.clone())?;

    let globals = lua.globals();
    globals.set("cranky", cranky_table)?;
    globals.set("ui", ui_table)?;
    globals.set("signals", signals_table)?;
    globals.set("config", config_table)?;
    globals.set("sys", sys_table)?;

    Ok(())
}

/// Legacy alias for `register_cranky_api`.
///
/// # Errors
///
/// Returns `mlua::Error` if table creation or function registration fails.
pub fn register_vdom_dsl(lua: &Lua) -> mlua::Result<()> {
    register_cranky_api(lua)
}
