use crate::features::module_runtime::ports::ModuleInitError;
use crate::features::styling::domain::StyleSheetName;
use crate::shared::config::domain::{ModuleConfig, RootConfig};
use crate::shared::dbus::domain::{DBusSubscription, Destination, Interface, Member, Path};
use crate::shared::events::signals::SignalKind;
use crate::shared::scripting::adapters::subscriptions::{
    bus_type_named, signal_kind_named, with_default_style,
};
use mlua::{Function, Lua, LuaSerdeExt};

pub(super) fn setup_config_tables(
    lua: &Lua,
    config: &ModuleConfig,
    full_config: &crate::shared::config::domain::Config,
) -> Result<(), ModuleInitError> {
    let globals = lua.globals();
    let config_table = config_table(lua, &globals).map_err(|error| script_error(&error))?;
    let root_config_table =
        root_config_table(lua, full_config.root()).map_err(|error| script_error(&error))?;
    config_table
        .set("root", root_config_table)
        .map_err(|error| script_error(&error))?;

    let options_lua = lua.to_value(config.options()).map_err(|error| {
        ModuleInitError::ConfigError(format!("Failed to convert config to Lua: {error}"))
    })?;
    config_table
        .set("module", options_lua.clone())
        .map_err(|error| script_error(&error))?;
    config_table
        .set("options", options_lua)
        .map_err(|error| script_error(&error))?;

    Ok(())
}

/// A Lua error as the script error a module init reports.
fn script_error(error: &mlua::Error) -> ModuleInitError {
    ModuleInitError::ScriptError(error.to_string())
}

/// The `config` global of the state, created and set when the script has none.
fn config_table(lua: &Lua, globals: &mlua::Table) -> mlua::Result<mlua::Table> {
    if let Some(table) = globals.get::<Option<mlua::Table>>("config")? {
        return Ok(table);
    }
    let table = lua.create_table()?;
    globals.set("config", table.clone())?;
    Ok(table)
}

/// The `config.root` table: the bar's name and height.
fn root_config_table(lua: &Lua, root_config: &RootConfig) -> mlua::Result<mlua::Table> {
    let root_config_table = lua.create_table()?;
    root_config_table.set("name", root_config.name().as_str())?;
    root_config_table.set("height", root_config.height().value())?;
    Ok(root_config_table)
}

pub(super) fn evaluate_metadata(
    lua: &Lua,
    module_name: &str,
) -> (Vec<SignalKind>, Vec<DBusSubscription>, Vec<StyleSheetName>) {
    let globals = lua.globals();
    let mut subs = Vec::new();
    let mut dbus_subs = Vec::new();
    let mut styles = Vec::new();

    if let Some(metadata_table) = call_for_table(&globals, "metadata") {
        if let Some(subs_table) = table_field(&metadata_table, "subscriptions") {
            parse_subscriptions_table(&subs_table, &mut subs, &mut dbus_subs);
        }
        if let Some(styles_table) = table_field(&metadata_table, "styles") {
            styles = parse_styles_table(&styles_table);
        }
    } else if let Some(subscriptions_table) = call_for_table(&globals, "subscriptions") {
        parse_subscriptions_table(&subscriptions_table, &mut subs, &mut dbus_subs);
    }

    (subs, dbus_subs, with_default_style(styles, module_name))
}

/// The table the global function `name` returns, if the script defines it and it returns one.
fn call_for_table(globals: &mlua::Table, name: &str) -> Option<mlua::Table> {
    let function = globals.get::<Function>(name).ok()?;
    match function.call::<mlua::Value>(()) {
        Ok(mlua::Value::Table(table)) => Some(table),
        _ => None,
    }
}

/// The field `key` of `table`, if it is a table.
fn table_field(table: &mlua::Table, key: &str) -> Option<mlua::Table> {
    match table.get::<mlua::Value>(key) {
        Ok(mlua::Value::Table(field)) => Some(field),
        _ => None,
    }
}

/// The valid style sheet names among the strings of `styles_table`.
fn parse_styles_table(styles_table: &mlua::Table) -> Vec<StyleSheetName> {
    styles_table
        .pairs::<mlua::Value, mlua::Value>()
        .flatten()
        .filter_map(|(_, val)| {
            let mlua::Value::String(lua_string) = val else {
                return None;
            };
            let s_str = lua_string.to_str().ok()?;
            StyleSheetName::new(s_str.as_ref()).ok()
        })
        .collect()
}

fn parse_subscriptions_table(
    table: &mlua::Table,
    subs: &mut Vec<SignalKind>,
    dbus_subs: &mut Vec<DBusSubscription>,
) {
    for (_, val) in table.pairs::<mlua::Value, mlua::Value>().flatten() {
        if let mlua::Value::String(lua_string) = &val {
            if let Some(signal) = lua_string
                .to_str()
                .ok()
                .and_then(|name| signal_kind_named(&name))
            {
                subs.push(signal);
            }
        } else if let mlua::Value::Table(dbus_sub) = &val
            && let Some(subscription) = parse_dbus_subscription(dbus_sub)
        {
            subs.push(SignalKind::DBus);
            dbus_subs.push(subscription);
        }
    }
}

/// The `DBus` subscription `table` describes, if its `type` is `"dbus"`.
fn parse_dbus_subscription(table: &mlua::Table) -> Option<DBusSubscription> {
    if string_field(table, "type").as_deref() != Some("dbus") {
        return None;
    }
    let bus = bus_type_named(string_field(table, "bus").as_deref());
    Some(
        DBusSubscription::new(bus)
            .with_destination(string_field(table, "destination").map(Destination::new))
            .with_path(string_field(table, "path").map(Path::new))
            .with_interface(string_field(table, "interface").map(Interface::new))
            .with_member(string_field(table, "member").map(Member::new)),
    )
}

/// The field `key` of `table` as a string, if it reads as one.
fn string_field(table: &mlua::Table, key: &str) -> Option<String> {
    table.get::<String>(key).ok()
}
