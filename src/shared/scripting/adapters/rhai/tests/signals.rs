//! What a Rhai module sees after `init` and `refresh`: its `DBus` subscriptions, styles, signals
//! and monitors.
use crate::features::module_runtime::ports::AnyModulePort;
use crate::features::mpris::domain::{MprisState, PlayerName};
use crate::features::workspaces::domain::MonitorName;
use crate::shared::config::domain::{Config, ModuleConfig};
use crate::shared::dbus::domain::BusType;
use crate::shared::dbus::domain::{DBusState, DBusValue};
use crate::shared::events::signals::HyprlandState;
use crate::shared::events::signals::{SignalHub, SignalKind};
use crate::shared::primitives::MonitorId;
use crate::shared::primitives::geometry::Scale;
use crate::shared::scripting::adapters::rhai::RhaiModule;
use crate::shared::scripting::adapters::test_support::{FULL_DBUS_FIELDS, dbus_fields};
use std::collections::{BTreeMap, HashMap};

fn init_module(name: &str, source: &str) -> RhaiModule {
    let mut module = RhaiModule::new(name.into(), source).unwrap();
    let mod_config = ModuleConfig::new(
        name.into(),
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

const DBUS_SOURCE: &str = r#"
        fn metadata() {
            #{ subscriptions: [
                #{ type: "dbus", bus: "system", destination: "org.example.Dest",
                   path: "/org/example/Path", interface: "org.example.Iface", member: "Changed" },
                #{ type: "dbus" },
                #{ type: "other" }
            ] }
        }
        fn render(monitor) { ui.text("") }
    "#;

#[test]
fn test_rhai_dbus_subscription_carries_every_field() {
    let module = init_module("dbus_watch", DBUS_SOURCE);
    let [full, bare] = module.dbus_subscriptions() else {
        panic!(
            "expected two subscriptions, got {:?}",
            module.dbus_subscriptions()
        );
    };
    assert_eq!(dbus_fields(full), FULL_DBUS_FIELDS);
    assert_eq!(dbus_fields(bare), (BusType::Session, [None; 4]));
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

/// The MPRIS state with `player` active.
fn mpris_playing(player: &str) -> MprisState {
    MprisState {
        active_player: Some(PlayerName::new(player)),
        players: HashMap::new(),
    }
}

/// A hub whose Hyprland focus, MPRIS player and `DBus` properties are set, with DP-1 at scale 2.
fn hub_with_signals() -> SignalHub {
    let hub = SignalHub::new(Config::default());
    let focus = Some(MonitorName::new("DP-7"));
    hub.hyprland_tx()
        .send(HyprlandState::new(BTreeMap::new(), BTreeMap::new(), focus))
        .unwrap();
    hub.mpris_tx().send(mpris_playing("spotify")).unwrap();
    let volume = ("org.example.Volume".to_string(), DBusValue::Int(42));
    hub.dbus_tx()
        .send(DBusState::new(HashMap::from([volume])))
        .unwrap();
    hub.monitor_scales_tx()
        .send(HashMap::from([(MonitorId::new("DP-1"), Scale::new(2.0))]))
        .unwrap();
    hub
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
    let hub = hub_with_signals();

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
    hub.mpris_tx().send(mpris_playing("vlc")).unwrap();

    module.refresh(&hub, &[SignalKind::Mpris]);

    assert_eq!(rendered_text(&module, "DP-1"), "vlc");
}
