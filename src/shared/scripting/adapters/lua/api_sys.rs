use super::userdata::LuaMonitor;
use mlua::Lua;

macro_rules! register_lua_log_methods {
    ($lua:expr, $table:expr, $(($name:literal, $level:ident)),*) => {
        $(
            $table.set(
                $name,
                $lua.create_function(|_, msg: String| {
                    tracing::$level!("{msg}");
                    Ok(())
                })?,
            )?;
        )*
    };
}

fn create_sys_log_table(lua: &Lua) -> mlua::Result<mlua::Table> {
    let log_table = lua.create_table()?;
    register_lua_log_methods!(
        lua,
        log_table,
        ("debug", debug),
        ("info", info),
        ("warn", warn),
        ("error", error)
    );
    Ok(log_table)
}

fn create_sys_monitors_table(lua: &Lua) -> mlua::Result<mlua::Table> {
    let monitors_table = lua.create_table()?;
    monitors_table.set("all", lua.create_function(all_monitors)?)?;
    monitors_table.set("focused", lua.create_function(focused_monitor)?)?;
    monitors_table.set("current", lua.create_function(focused_monitor)?)?;
    monitors_table.set(
        "get",
        lua.create_function(|lua, id: String| monitor_by_id(lua, &id))?,
    )?;
    Ok(monitors_table)
}

/// `sys.monitors.all()`: the monitors the last refresh listed, or an empty table before it.
fn all_monitors(lua: &Lua, (): ()) -> mlua::Result<mlua::Table> {
    let globals = lua.globals();
    let raw_monitors = globals.get::<Option<mlua::Table>>("_raw_monitors")?;
    if let Some(monitors) = raw_monitors {
        return Ok(monitors);
    }
    lua.create_table()
}

/// `sys.monitors.focused()` and `sys.monitors.current()`: the monitor the last refresh found
/// focused, if any.
fn focused_monitor(lua: &Lua, (): ()) -> mlua::Result<Option<mlua::Value>> {
    let globals = lua.globals();
    let raw_focused = globals.get::<Option<mlua::Value>>("_raw_focused_monitor")?;
    Ok(raw_focused)
}

/// `sys.monitors.get(id)`: the first monitor the last refresh listed with the id `id`, if any.
fn monitor_by_id(lua: &Lua, id: &str) -> mlua::Result<Option<mlua::Value>> {
    let globals = lua.globals();
    let raw_monitors = globals.get::<Option<mlua::Table>>("_raw_monitors")?;
    let Some(monitors) = raw_monitors else {
        return Ok(None);
    };
    for pair in monitors.sequence_values::<mlua::Value>() {
        if let Ok(mlua::Value::UserData(user_data)) = pair
            && let Ok(mon) = user_data.borrow::<LuaMonitor>()
            && mon.0.id().as_str() == id
        {
            return Ok(Some(mlua::Value::UserData(user_data)));
        }
    }
    Ok(None)
}

pub(super) fn create_sys_table(lua: &Lua) -> mlua::Result<mlua::Table> {
    let sys = lua.create_table()?;
    sys.set(
        "exec",
        lua.create_function(|_, cmd: String| {
            run_shell_command(&cmd);
            Ok(())
        })?,
    )?;
    sys.set(
        "env",
        lua.create_function(|_, name: String| Ok(std::env::var(&name).ok()))?,
    )?;
    sys.set(
        "hostname",
        lua.create_function(|_, ()| Ok(local_hostname()))?,
    )?;

    let log_table = create_sys_log_table(lua)?;
    sys.set("log", log_table)?;

    let monitors_table = create_sys_monitors_table(lua)?;
    sys.set("monitors", monitors_table)?;

    Ok(sys)
}

/// Starts `cmd` with `sh -c` without waiting for it; a failed start is ignored.
fn run_shell_command(cmd: &str) {
    let _ = std::process::Command::new("sh").arg("-c").arg(cmd).spawn();
}

/// `/etc/hostname` trimmed, else `$HOSTNAME`, else `"localhost"`.
fn local_hostname() -> String {
    std::fs::read_to_string("/etc/hostname").map_or_else(
        |_| std::env::var("HOSTNAME").unwrap_or_else(|_| "localhost".to_string()),
        |contents| contents.trim().to_string(),
    )
}
