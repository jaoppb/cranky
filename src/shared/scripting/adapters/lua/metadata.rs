use crate::features::module_runtime::ports::ModuleInitError;
use crate::shared::config::domain::ModuleConfig;
use crate::shared::dbus::domain::{BusType, DBusSubscription};
use crate::shared::events::signals::SignalKind;
use mlua::{Function, Lua, LuaSerdeExt};

pub(super) fn setup_config_tables(
    lua: &Lua,
    config: &ModuleConfig,
    full_config: &crate::shared::config::domain::Config,
) -> Result<(), ModuleInitError> {
    let globals = lua.globals();
    let root_config = full_config.root();

    let config_table = if let Some(table) = globals
        .get::<Option<mlua::Table>>("config")
        .map_err(|error| ModuleInitError::ScriptError(error.to_string()))?
    {
        table
    } else {
        let table = lua
            .create_table()
            .map_err(|error| ModuleInitError::ScriptError(error.to_string()))?;
        globals
            .set("config", table.clone())
            .map_err(|error| ModuleInitError::ScriptError(error.to_string()))?;
        table
    };

    let root_config_table = lua
        .create_table()
        .map_err(|error| ModuleInitError::ScriptError(error.to_string()))?;
    root_config_table
        .set("name", root_config.name().as_str())
        .map_err(|error| ModuleInitError::ScriptError(error.to_string()))?;
    root_config_table
        .set("height", root_config.height().value())
        .map_err(|error| ModuleInitError::ScriptError(error.to_string()))?;
    config_table
        .set("root", root_config_table)
        .map_err(|error| ModuleInitError::ScriptError(error.to_string()))?;

    let options_lua = lua.to_value(config.options()).map_err(|error| {
        ModuleInitError::ConfigError(format!("Failed to convert config to Lua: {error}"))
    })?;
    config_table
        .set("module", options_lua.clone())
        .map_err(|error| ModuleInitError::ScriptError(error.to_string()))?;
    config_table
        .set("options", options_lua)
        .map_err(|error| ModuleInitError::ScriptError(error.to_string()))?;

    Ok(())
}

pub(super) fn evaluate_metadata(
    lua: &Lua,
    module_name: &str,
) -> (
    Vec<SignalKind>,
    Vec<DBusSubscription>,
    Vec<crate::features::styling::domain::StyleSheetName>,
) {
    let globals = lua.globals();
    let mut subs = Vec::new();
    let mut dbus_subs = Vec::new();
    let mut styles = Vec::new();

    if let Ok(meta_fn) = globals.get::<Function>("metadata")
        && let Ok(result) = meta_fn.call::<mlua::Value>(())
        && let mlua::Value::Table(metadata_table) = result
    {
        if let Ok(subs_val) = metadata_table.get::<mlua::Value>("subscriptions")
            && let mlua::Value::Table(subs_table) = subs_val
        {
            parse_subscriptions_table(&subs_table, &mut subs, &mut dbus_subs);
        }

        if let Ok(styles_val) = metadata_table.get::<mlua::Value>("styles")
            && let mlua::Value::Table(styles_table) = styles_val
        {
            for (_, val) in styles_table.pairs::<mlua::Value, mlua::Value>().flatten() {
                if let mlua::Value::String(lua_string) = val
                    && let Ok(s_str) = lua_string.to_str()
                    && let Ok(sheet) =
                        crate::features::styling::domain::StyleSheetName::new(s_str.as_ref())
                {
                    styles.push(sheet);
                }
            }
        }
    } else if let Ok(subs_fn) = globals.get::<Function>("subscriptions")
        && let Ok(result) = subs_fn.call::<mlua::Value>(())
        && let mlua::Value::Table(subscriptions_table) = result
    {
        parse_subscriptions_table(&subscriptions_table, &mut subs, &mut dbus_subs);
    }

    if styles.is_empty()
        && let Ok(default_sheet) =
            crate::features::styling::domain::StyleSheetName::new(module_name)
    {
        styles.push(default_sheet);
    }

    (subs, dbus_subs, styles)
}

fn parse_subscriptions_table(
    table: &mlua::Table,
    subs: &mut Vec<SignalKind>,
    dbus_subs: &mut Vec<DBusSubscription>,
) {
    for (_, val) in table.pairs::<mlua::Value, mlua::Value>().flatten() {
        if let mlua::Value::String(lua_string) = &val {
            if let Ok(s_str) = lua_string.to_str() {
                match s_str.as_ref() {
                    "time" => subs.push(SignalKind::Time),
                    "hyprland" => subs.push(SignalKind::Hyprland),
                    "systray" => subs.push(SignalKind::Systray),
                    "metrics" => subs.push(SignalKind::Metrics),
                    "mpris" => subs.push(SignalKind::Mpris),
                    _ => {}
                }
            }
        } else if let mlua::Value::Table(dbus_sub) = &val
            && let Ok(typ) = dbus_sub.get::<String>("type")
            && typ == "dbus"
        {
            let bus_str = dbus_sub
                .get::<String>("bus")
                .unwrap_or_else(|_| "session".to_string());
            let bus = if bus_str == "system" {
                BusType::System
            } else {
                BusType::Session
            };
            subs.push(SignalKind::DBus);
            dbus_subs.push(DBusSubscription::new(
                bus,
                dbus_sub
                    .get::<String>("destination")
                    .ok()
                    .map(crate::shared::dbus::domain::Destination::new),
                dbus_sub
                    .get::<String>("path")
                    .ok()
                    .map(crate::shared::dbus::domain::Path::new),
                dbus_sub
                    .get::<String>("interface")
                    .ok()
                    .map(crate::shared::dbus::domain::Interface::new),
                dbus_sub
                    .get::<String>("member")
                    .ok()
                    .map(crate::shared::dbus::domain::Member::new),
            ));
        }
    }
}
