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

pub(crate) fn create_sys_log_table(lua: &Lua) -> mlua::Result<mlua::Table> {
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

pub(crate) fn create_sys_monitors_table(lua: &Lua) -> mlua::Result<mlua::Table> {
    let monitors_table = lua.create_table()?;
    monitors_table.set(
        "all",
        lua.create_function(|lua, ()| {
            let globals = lua.globals();
            let raw_monitors = globals.get::<Option<mlua::Table>>("_raw_monitors")?;
            if let Some(t) = raw_monitors {
                return Ok(t);
            }
            lua.create_table()
        })?,
    )?;
    monitors_table.set(
        "focused",
        lua.create_function(|lua, ()| {
            let globals = lua.globals();
            let raw_focused = globals.get::<Option<mlua::Value>>("_raw_focused_monitor")?;
            Ok(raw_focused)
        })?,
    )?;
    monitors_table.set(
        "current",
        lua.create_function(|lua, ()| {
            let globals = lua.globals();
            let raw_focused = globals.get::<Option<mlua::Value>>("_raw_focused_monitor")?;
            Ok(raw_focused)
        })?,
    )?;
    monitors_table.set(
        "get",
        lua.create_function(|lua, id: String| {
            let globals = lua.globals();
            let raw_monitors = globals.get::<Option<mlua::Table>>("_raw_monitors")?;
            let Some(t) = raw_monitors else {
                return Ok(None);
            };
            for pair in t.sequence_values::<mlua::Value>() {
                if let Ok(mlua::Value::UserData(ud)) = pair
                    && let Ok(mon) = ud.borrow::<LuaMonitor>()
                    && mon.0.id().as_str() == id.as_str()
                {
                    return Ok(Some(mlua::Value::UserData(ud)));
                }
            }
            Ok(None)
        })?,
    )?;
    Ok(monitors_table)
}

pub(crate) fn create_sys_table(lua: &Lua) -> mlua::Result<mlua::Table> {
    let sys = lua.create_table()?;

    sys.set(
        "exec",
        lua.create_function(|_, cmd: String| {
            let _ = std::process::Command::new("sh").arg("-c").arg(&cmd).spawn();
            Ok(())
        })?,
    )?;

    sys.set(
        "env",
        lua.create_function(|_, name: String| Ok(std::env::var(&name).ok()))?,
    )?;

    sys.set(
        "hostname",
        lua.create_function(|_, ()| {
            let name = std::fs::read_to_string("/etc/hostname").map_or_else(
                |_| std::env::var("HOSTNAME").unwrap_or_else(|_| "localhost".to_string()),
                |s| s.trim().to_string(),
            );
            Ok(name)
        })?,
    )?;

    let log_table = create_sys_log_table(lua)?;
    sys.set("log", log_table)?;

    let monitors_table = create_sys_monitors_table(lua)?;
    sys.set("monitors", monitors_table)?;

    Ok(sys)
}
