//! What a Lua module's `metadata()` subscribes to.
use crate::features::module_runtime::ports::AnyModulePort;
use crate::shared::config::domain::{Config, ModuleConfig};
use crate::shared::dbus::domain::BusType;
use crate::shared::events::signals::SignalKind;
use crate::shared::scripting::adapters::lua::LuaModule;

#[test]
fn test_lua_dbus_subscription_carries_every_field() {
    let source = r#"
        function metadata()
            return { subscriptions = {
                { type = "dbus", bus = "system", destination = "org.example.Dest",
                  path = "/org/example/Path", interface = "org.example.Iface", member = "Changed" },
                { type = "dbus" },
                { type = "other" },
            } }
        end
        function render(monitor) return ui.text("") end
    "#;
    let mut module = LuaModule::new("dbus_watch".to_string(), source.to_string());
    let mod_config = ModuleConfig::new(
        "dbus_watch".into(),
        crate::shared::primitives::ModuleOptions::default(),
    );
    module.init(&mod_config, &Config::default()).unwrap();

    let [full, bare] = module.dbus_subscriptions() else {
        panic!(
            "expected two subscriptions, got {:?}",
            module.dbus_subscriptions()
        );
    };
    assert_eq!(full.bus(), BusType::System);
    assert_eq!(
        full.destination()
            .map(crate::shared::dbus::domain::Destination::as_str),
        Some("org.example.Dest")
    );
    assert_eq!(
        full.path().map(crate::shared::dbus::domain::Path::as_str),
        Some("/org/example/Path")
    );
    assert_eq!(
        full.interface()
            .map(crate::shared::dbus::domain::Interface::as_str),
        Some("org.example.Iface")
    );
    assert_eq!(
        full.member()
            .map(crate::shared::dbus::domain::Member::as_str),
        Some("Changed")
    );
    assert_eq!(bare.bus(), BusType::Session);
    assert!(bare.destination().is_none() && bare.path().is_none());
    assert!(bare.interface().is_none() && bare.member().is_none());
    assert_eq!(module.subscriptions(), [SignalKind::DBus, SignalKind::DBus]);
}

/// A Lua module running `source`, initialised with the default configuration.
fn init_module(name: &str, source: &str) -> LuaModule {
    let mut module = LuaModule::new(name.to_string(), source.to_string());
    let mod_config = ModuleConfig::new(
        name.into(),
        crate::shared::primitives::ModuleOptions::default(),
    );
    module.init(&mod_config, &Config::default()).unwrap();
    module
}

fn style_names(module: &LuaModule) -> Vec<&str> {
    module
        .styles()
        .iter()
        .map(crate::features::styling::domain::StyleSheetName::as_str)
        .collect()
}

#[test]
fn test_lua_metadata_subscriptions_name_each_signal_and_skip_others() {
    let module = init_module(
        "named",
        r#"function metadata()
            return { subscriptions = { "time", "hyprland", "systray", "metrics", "mpris",
                                       "dbus", "weather", 7 } }
        end"#,
    );
    assert_eq!(
        module.subscriptions(),
        [
            SignalKind::Time,
            SignalKind::Hyprland,
            SignalKind::Systray,
            SignalKind::Metrics,
            SignalKind::Mpris
        ]
    );
    assert_eq!(module.dbus_subscriptions(), []);
}

#[test]
fn test_lua_metadata_styles_keep_valid_names_and_fall_back_to_the_module_name() {
    let styled = init_module(
        "styled",
        r#"function metadata() return { styles = { 42, "custom", "../escape", "extra" } } end"#,
    );
    assert_eq!(style_names(&styled), ["custom", "extra"]);
    let invalid = init_module(
        "invalid",
        r#"function metadata() return { styles = { "a/b" }, subscriptions = "time" } end"#,
    );
    assert_eq!(style_names(&invalid), ["invalid"]);
    assert_eq!(invalid.subscriptions(), []);
    let bare = init_module("bare", "");
    assert_eq!(style_names(&bare), ["bare"]);
    assert!(bare.subscriptions().is_empty() && bare.dbus_subscriptions().is_empty());
}

#[test]
fn test_lua_subscriptions_function_is_read_without_a_metadata_table() {
    for source in [
        r#"function subscriptions() return { "time", { type = "dbus" } } end"#,
        r#"function metadata() return "none" end
           function subscriptions() return { "time", { type = "dbus" } } end"#,
    ] {
        let module = init_module("legacy", source);
        assert_eq!(module.subscriptions(), [SignalKind::Time, SignalKind::DBus]);
        assert_eq!(module.dbus_subscriptions().len(), 1);
        assert_eq!(style_names(&module), ["legacy"]);
    }
}

#[test]
fn test_lua_subscriptions_function_is_ignored_beside_a_metadata_table() {
    let module = init_module(
        "both",
        r#"function metadata() return { subscriptions = { "mpris" } } end
           function subscriptions() return { "time" } end"#,
    );
    assert_eq!(module.subscriptions(), [SignalKind::Mpris]);
}

#[test]
fn test_lua_dbus_subscription_bus_other_than_system_is_the_session_bus() {
    let module = init_module(
        "buses",
        r#"function metadata() return { subscriptions = {
            { type = "dbus", bus = "System" }, { type = "dbus", bus = 5 }, { type = "dbus", bus = "system" },
        } } end"#,
    );
    let buses: Vec<BusType> = module
        .dbus_subscriptions()
        .iter()
        .map(crate::shared::dbus::domain::DBusSubscription::bus)
        .collect();
    assert_eq!(buses, [BusType::Session, BusType::Session, BusType::System]);
}

#[test]
fn test_lua_subscription_names_that_are_no_utf8_are_skipped() {
    let module = init_module(
        "bytes",
        r#"function metadata() return { subscriptions = { "\xff", "time" } } end"#,
    );
    assert_eq!(module.subscriptions(), [SignalKind::Time]);
}
