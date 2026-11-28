use super::script::RhaiScript;
use crate::features::module_runtime::ports::ModuleInitError;
use crate::features::styling::domain::StyleSheetName;
use crate::shared::config::domain::{ModuleConfig, RootConfig};
use crate::shared::dbus::domain::{DBusSubscription, Destination, Interface, Member, Path};
use crate::shared::events::signals::SignalKind;
use crate::shared::scripting::adapters::subscriptions::{
    bus_type_named, signal_kind_named, with_default_style,
};
use rhai::Dynamic;

pub(super) fn setup_config_maps(
    script: &mut RhaiScript<'_>,
    config: &ModuleConfig,
    full_config: &crate::shared::config::domain::Config,
) -> Result<(), ModuleInitError> {
    let root_map = root_config_map(full_config.root());
    let options_rhai = options_map(script, config)?;

    let scope = script.scope_mut();
    let mut config_map = scope.get_value::<rhai::Map>("config").unwrap_or_default();
    config_map.insert("module".into(), Dynamic::from(options_rhai.clone()));
    config_map.insert("options".into(), Dynamic::from(options_rhai));
    config_map.insert("root".into(), Dynamic::from(root_map));

    scope.set_value("config", config_map.clone());

    if let Some(mut cranky_map) = scope.get_value::<rhai::Map>("cranky") {
        cranky_map.insert("config".into(), Dynamic::from(config_map));
        scope.set_value("cranky", cranky_map);
    }

    Ok(())
}

/// The `config.root` map: the bar's name and height.
fn root_config_map(root_config: &RootConfig) -> rhai::Map {
    let mut root_map = rhai::Map::new();
    root_map.insert(
        "name".into(),
        Dynamic::from(root_config.name().as_str().to_string()),
    );
    root_map.insert(
        "height".into(),
        Dynamic::from(i64::from(root_config.height().value())),
    );
    root_map
}

/// The module's options as the map their JSON parses to.
fn options_map(
    script: &RhaiScript<'_>,
    config: &ModuleConfig,
) -> Result<rhai::Map, ModuleInitError> {
    let options_json = serde_json::to_string(config.options())
        .map_err(|error| ModuleInitError::ConfigError(error.to_string()))?;
    script
        .engine()
        .parse_json(&options_json, true)
        .map_err(|error| ModuleInitError::ScriptError(error.to_string()))
}

/// The field `key` of `map`, if it is a string.
fn string_field(map: &rhai::Map, key: &str) -> Option<String> {
    map.get(key)
        .and_then(|value| value.clone().try_cast::<String>())
}

/// The field `key` of `map`, if it is an array.
fn array_field(map: &rhai::Map, key: &str) -> Option<rhai::Array> {
    map.get(key)
        .and_then(|value| value.clone().try_cast::<rhai::Array>())
}

/// The `DBus` subscription `map` describes, if its `type` is `"dbus"`.
fn parse_dbus_subscription(map: &rhai::Map) -> Option<DBusSubscription> {
    if string_field(map, "type").as_deref() != Some("dbus") {
        return None;
    }
    let bus = bus_type_named(string_field(map, "bus").as_deref());
    Some(
        DBusSubscription::new(bus)
            .with_destination(string_field(map, "destination").map(Destination::new))
            .with_path(string_field(map, "path").map(Path::new))
            .with_interface(string_field(map, "interface").map(Interface::new))
            .with_member(string_field(map, "member").map(Member::new)),
    )
}

fn parse_subscriptions_array(
    subs: &rhai::Array,
    result: &mut Vec<SignalKind>,
    dbus_subs: &mut Vec<DBusSubscription>,
) {
    for sub in subs {
        if let Some(signal_name) = sub.clone().try_cast::<String>() {
            result.extend(signal_kind_named(&signal_name));
        } else if let Some(map) = sub.clone().try_cast::<rhai::Map>()
            && let Some(dbus_sub) = parse_dbus_subscription(&map)
        {
            result.push(SignalKind::DBus);
            dbus_subs.push(dbus_sub);
        }
    }
}

/// The valid style sheet names among the strings of `styles_arr`.
fn parse_styles_array(styles_arr: rhai::Array) -> Vec<StyleSheetName> {
    styles_arr
        .into_iter()
        .filter_map(Dynamic::try_cast::<String>)
        .filter_map(|str_val| StyleSheetName::new(str_val).ok())
        .collect()
}

pub(super) fn evaluate_metadata(
    script: &mut RhaiScript<'_>,
    module_name: &str,
) -> (Vec<SignalKind>, Vec<DBusSubscription>, Vec<StyleSheetName>) {
    let mut subs = Vec::new();
    let mut dbus_subs = Vec::new();
    let mut styles = Vec::new();

    let (engine, scope, ast) = script.parts();
    if let Ok(meta) = engine.call_fn::<rhai::Map>(scope, ast, "metadata", ()) {
        if let Some(subs_arr) = array_field(&meta, "subscriptions") {
            parse_subscriptions_array(&subs_arr, &mut subs, &mut dbus_subs);
        }
        if let Some(styles_arr) = array_field(&meta, "styles") {
            styles = parse_styles_array(styles_arr);
        }
    } else if let Ok(subs_arr) = engine.call_fn::<rhai::Array>(scope, ast, "subscriptions", ()) {
        parse_subscriptions_array(&subs_arr, &mut subs, &mut dbus_subs);
    }

    (subs, dbus_subs, with_default_style(styles, module_name))
}
