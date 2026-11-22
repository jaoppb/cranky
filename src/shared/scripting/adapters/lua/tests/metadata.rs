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
        crate::shared::config::domain::Enablement::Enabled,
        crate::shared::config::domain::EngineSelection::Auto,
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
