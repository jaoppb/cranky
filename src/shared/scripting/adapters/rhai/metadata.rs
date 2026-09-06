use crate::features::module_runtime::ports::ModuleInitError;
use crate::shared::config::domain::ModuleConfig;
use crate::shared::events::signals::SignalKind;
use rhai::{AST, Dynamic, Engine, Scope};

pub(crate) fn setup_config_maps(
    scope: &mut Scope<'static>,
    engine: &Engine,
    config: &ModuleConfig,
    full_config: &crate::shared::config::domain::Config,
) -> Result<(), ModuleInitError> {
    let root_config = full_config.root();

    let mut root_map = rhai::Map::new();
    root_map.insert(
        "name".into(),
        Dynamic::from(root_config.name().as_str().to_string()),
    );
    root_map.insert(
        "height".into(),
        Dynamic::from(i64::from(root_config.height().value())),
    );

    let options_json = serde_json::to_string(config.options())
        .map_err(|e| ModuleInitError::ConfigError(e.to_string()))?;
    let options_rhai: rhai::Map = engine
        .parse_json(&options_json, true)
        .map_err(|e| ModuleInitError::ScriptError(e.to_string()))?;

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

pub(crate) fn parse_dbus_subscription(
    map: &rhai::Map,
) -> Option<crate::shared::dbus::domain::DBusSubscription> {
    let is_dbus = map
        .get("type")
        .and_then(|v| v.clone().try_cast::<String>())
        .as_deref()
        == Some("dbus");
    if !is_dbus {
        return None;
    }

    let bus = match map
        .get("bus")
        .and_then(|v| v.clone().try_cast::<String>())
        .as_deref()
    {
        Some("system") => crate::shared::dbus::domain::BusType::System,
        _ => crate::shared::dbus::domain::BusType::Session,
    };

    Some(crate::shared::dbus::domain::DBusSubscription::new(
        bus,
        map.get("destination")
            .and_then(|v| v.clone().try_cast::<String>())
            .map(crate::shared::dbus::domain::Destination::new),
        map.get("path")
            .and_then(|v| v.clone().try_cast::<String>())
            .map(crate::shared::dbus::domain::Path::new),
        map.get("interface")
            .and_then(|v| v.clone().try_cast::<String>())
            .map(crate::shared::dbus::domain::Interface::new),
        map.get("member")
            .and_then(|v| v.clone().try_cast::<String>())
            .map(crate::shared::dbus::domain::Member::new),
    ))
}

pub(crate) fn parse_subscriptions_array(
    subs: &rhai::Array,
    result: &mut Vec<SignalKind>,
    dbus_subs: &mut Vec<crate::shared::dbus::domain::DBusSubscription>,
) {
    for sub in subs {
        if let Some(s) = sub.clone().try_cast::<String>() {
            match s.as_str() {
                "time" => result.push(SignalKind::Time),
                "hyprland" => result.push(SignalKind::Hyprland),
                "systray" => result.push(SignalKind::Systray),
                "metrics" => result.push(SignalKind::Metrics),
                "mpris" => result.push(SignalKind::Mpris),
                _ => {}
            }
        } else if let Some(map) = sub.clone().try_cast::<rhai::Map>()
            && let Some(dbus_sub) = parse_dbus_subscription(&map)
        {
            result.push(SignalKind::DBus);
            dbus_subs.push(dbus_sub);
        }
    }
}

pub(crate) fn evaluate_metadata(
    engine: &Engine,
    scope: &mut Scope<'static>,
    ast: &AST,
    module_name: &str,
) -> (
    Vec<SignalKind>,
    Vec<crate::shared::dbus::domain::DBusSubscription>,
    Vec<crate::features::styling::domain::StyleSheetName>,
) {
    let mut subs = Vec::new();
    let mut dbus_subs = Vec::new();
    let mut styles = Vec::new();

    if let Ok(meta) = engine.call_fn::<rhai::Map>(scope, ast, "metadata", ()) {
        if let Some(subs_arr) = meta
            .get("subscriptions")
            .and_then(|v| v.clone().try_cast::<rhai::Array>())
        {
            parse_subscriptions_array(&subs_arr, &mut subs, &mut dbus_subs);
        }
        if let Some(styles_arr) = meta
            .get("styles")
            .and_then(|v| v.clone().try_cast::<rhai::Array>())
        {
            for s in styles_arr {
                if let Some(str_val) = s.try_cast::<String>()
                    && let Ok(sheet) =
                        crate::features::styling::domain::StyleSheetName::new(str_val)
                {
                    styles.push(sheet);
                }
            }
        }
    } else if let Ok(subs_arr) = engine.call_fn::<rhai::Array>(scope, ast, "subscriptions", ()) {
        parse_subscriptions_array(&subs_arr, &mut subs, &mut dbus_subs);
    }

    if styles.is_empty()
        && let Ok(default_sheet) = crate::features::styling::domain::StyleSheetName::new(module_name)
    {
        styles.push(default_sheet);
    }

    (subs, dbus_subs, styles)
}
