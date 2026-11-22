//! What a Rhai module sees after `init` and `refresh`: its `DBus` subscriptions, styles, signals
//! and monitors.
use crate::features::module_runtime::ports::AnyModulePort;
use crate::shared::config::domain::{Config, ModuleConfig};
use crate::shared::dbus::domain::BusType;
use crate::shared::events::signals::{SignalHub, SignalKind};
use crate::shared::primitives::MonitorId;
use crate::shared::scripting::adapters::rhai::RhaiModule;

fn init_module(name: &str, source: &str) -> RhaiModule {
    let mut module = RhaiModule::new(name.into(), source).unwrap();
    let mod_config = ModuleConfig::new(
        name.into(),
        crate::shared::config::domain::Enablement::Enabled,
        crate::shared::config::domain::EngineSelection::Auto,
        crate::shared::primitives::ModuleOptions::default(),
    );
    module.init(&mod_config, &Config::default()).unwrap();
    module
}

fn rendered_text(module: &RhaiModule, monitor: &str) -> String {
    let node = module.render(&MonitorId::new(monitor));
    let crate::features::vdom::domain::VNodeKind::Text { text } = node.kind() else {
        panic!("Expected a text node, got {:?}", node.tag());
    };
    text.as_str().to_string()
}

#[test]
fn test_rhai_dbus_subscription_carries_every_field() {
    let module = init_module(
        "dbus_watch",
        r#"
        fn metadata() {
            #{ subscriptions: [
                #{ type: "dbus", bus: "system", destination: "org.example.Dest",
                   path: "/org/example/Path", interface: "org.example.Iface", member: "Changed" },
                #{ type: "dbus" },
                #{ type: "other" }
            ] }
        }
        fn render(monitor) { ui.text("") }
    "#,
    );
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

#[test]
fn test_rhai_metadata_styles_and_their_fallback() {
    let styled = init_module(
        "styled",
        r#"fn metadata() { #{ styles: [42, "custom"] } } fn render(monitor) { ui.text("") }"#,
    );
    let names: Vec<&str> = styled
        .styles()
        .iter()
        .map(crate::features::styling::domain::StyleSheetName::as_str)
        .collect();
    assert_eq!(names, ["custom"]);
    assert_eq!(styled.subscriptions(), Vec::<SignalKind>::new());

    let bare = init_module("bare", r#"fn render(monitor) { ui.text("") }"#);
    let names: Vec<&str> = bare
        .styles()
        .iter()
        .map(crate::features::styling::domain::StyleSheetName::as_str)
        .collect();
    assert_eq!(names, ["bare"]);
    assert!(bare.subscriptions().is_empty() && bare.dbus_subscriptions().is_empty());
}

#[test]
fn test_rhai_refresh_publishes_every_changed_signal_and_the_monitors() {
    // The systray list is a JSON array, which `parse_json` rejects, so scripts never see it.
    // That is a pre-existing bug, tracked in #117 and out of scope for #99; it is pinned here so
    // the fix is a deliberate change.
    let mut module = init_module(
        "signals",
        r#"
        fn render(monitor) {
            ui.text(`${signals.hyprland.focused_monitor}|${type_of(signals.systray)}|${signals.mpris.active_player}|${signals.dbus["org.example.Volume"]}|${cranky.signals.mpris.active_player}|${sys.monitors._list.len()}|${monitor.scale}`)
        }
    "#,
    );
    let hub = SignalHub::new(Config::default());
    hub.hyprland_tx()
        .send(
            crate::shared::events::signals::hyprland::HyprlandState::new(
                std::collections::BTreeMap::new(),
                std::collections::BTreeMap::new(),
                Some(crate::features::workspaces::domain::MonitorName::new(
                    "DP-7",
                )),
            ),
        )
        .unwrap();
    hub.mpris_tx()
        .send(crate::features::mpris::domain::MprisState {
            active_player: Some(crate::features::mpris::domain::PlayerName::new("spotify")),
            players: std::collections::HashMap::new(),
        })
        .unwrap();
    hub.dbus_tx()
        .send(crate::shared::dbus::domain::DBusState::new(
            std::collections::HashMap::from([(
                "org.example.Volume".to_string(),
                crate::shared::dbus::domain::DBusValue::Int(42),
            )]),
        ))
        .unwrap();
    hub.monitor_scales_tx()
        .send(std::collections::HashMap::from([(
            MonitorId::new("DP-1"),
            crate::shared::primitives::geometry::Scale::new(2.0),
        )]))
        .unwrap();

    module.refresh(
        &hub,
        &[
            SignalKind::Hyprland,
            SignalKind::Systray,
            SignalKind::Mpris,
            SignalKind::DBus,
            SignalKind::DBus,
        ],
    );

    assert_eq!(
        rendered_text(&module, "DP-1"),
        "DP-7|()|spotify|42|spotify|1|2.0"
    );
}

#[test]
fn test_rhai_refresh_error_keeps_the_published_signals() {
    let mut module = init_module(
        "failing",
        r#"
        fn refresh() { throw "boom"; }
        fn render(monitor) { ui.text(`${signals.mpris.active_player}`) }
    "#,
    );
    let hub = SignalHub::new(Config::default());
    hub.mpris_tx()
        .send(crate::features::mpris::domain::MprisState {
            active_player: Some(crate::features::mpris::domain::PlayerName::new("vlc")),
            players: std::collections::HashMap::new(),
        })
        .unwrap();

    module.refresh(&hub, &[SignalKind::Mpris]);

    assert_eq!(rendered_text(&module, "DP-1"), "vlc");
}
