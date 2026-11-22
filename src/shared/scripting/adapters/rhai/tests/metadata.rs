//! What a Rhai module's `metadata()` or `subscriptions()` subscribes to.
use super::support::init_module;
use crate::features::module_runtime::ports::AnyModulePort;
use crate::shared::dbus::domain::BusType;
use crate::shared::events::signals::SignalKind;

#[test]
fn test_rhai_metadata_subscriptions_name_each_signal_and_skip_others() {
    let module = init_module(
        "named",
        r#"fn metadata() { #{ subscriptions: ["time", "hyprland", "systray", "metrics", "mpris",
                                              "dbus", "weather", 7, #{ bus: "system" }] } }"#,
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
fn test_rhai_subscriptions_function_is_read_without_a_metadata_map() {
    for source in [
        r#"fn subscriptions() { ["time", #{ type: "dbus" }] }"#,
        r#"fn metadata() { "none" } fn subscriptions() { ["time", #{ type: "dbus" }] }"#,
    ] {
        let module = init_module("legacy", source);
        assert_eq!(module.subscriptions(), [SignalKind::Time, SignalKind::DBus]);
        assert_eq!(module.dbus_subscriptions().len(), 1);
    }
}

#[test]
fn test_rhai_subscriptions_function_is_ignored_beside_a_metadata_map() {
    let module = init_module(
        "both",
        r#"fn metadata() { #{ subscriptions: ["mpris"] } } fn subscriptions() { ["time"] }"#,
    );
    assert_eq!(module.subscriptions(), [SignalKind::Mpris]);
}

#[test]
fn test_rhai_dbus_subscription_bus_other_than_system_is_the_session_bus() {
    let module = init_module(
        "buses",
        r#"fn metadata() { #{ subscriptions: [
            #{ type: "dbus", bus: "System" }, #{ type: "dbus", bus: 5 }, #{ type: "dbus", bus: "system" }
        ] } }"#,
    );
    let buses: Vec<BusType> = module
        .dbus_subscriptions()
        .iter()
        .map(crate::shared::dbus::domain::DBusSubscription::bus)
        .collect();
    assert_eq!(buses, [BusType::Session, BusType::Session, BusType::System]);
}

#[test]
fn test_rhai_metadata_styles_skip_invalid_names() {
    let module = init_module(
        "styled",
        r#"fn metadata() { #{ styles: ["../escape", "custom"], subscriptions: "time" } }"#,
    );
    let names: Vec<&str> = module
        .styles()
        .iter()
        .map(crate::features::styling::domain::StyleSheetName::as_str)
        .collect();
    assert_eq!(names, ["custom"]);
    assert_eq!(module.subscriptions(), []);
}
