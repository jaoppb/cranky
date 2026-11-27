//! How `ZbusMprisAdapter` loads the players already on the bus and follows names coming and
//! going.
use std::sync::Arc;

use tokio::sync::mpsc;

use super::fake_bus::{FakeBus, mpris_name, player_read, props, reaches, text};
use crate::features::mpris::adapters::zbus::ZbusMprisAdapter;
use crate::features::mpris::domain::{MprisState, PlayerName};
use crate::shared::config::domain::Config;
use crate::shared::dbus::domain::{BusType, Destination, NameOwnerChanged};
use crate::shared::dbus::ports::{DbusConnectionError, DbusConnectionPort};
use crate::shared::events::signals::SignalHub;

/// A bus where `vlc` and `mpv` answer `GetAll`, listing `names`, with its name stream open.
fn bus_listing(names: &[&str]) -> (FakeBus, mpsc::UnboundedSender<NameOwnerChanged>) {
    let mut bus = FakeBus {
        names: Some(names.iter().map(|name| Destination::new(*name)).collect()),
        ..FakeBus::default()
    };
    for short in ["vlc", "mpv"] {
        let _changes = bus.with_player(&mpris_name(short), props(vec![("Identity", text(short))]));
    }
    let events = bus.with_name_changes();
    (bus, events)
}

/// Starts the adapter on `bus` with a fresh hub.
async fn started(bus: FakeBus) -> (Arc<FakeBus>, SignalHub, Result<(), DbusConnectionError>) {
    let bus = Arc::new(bus);
    let conn: Arc<dyn DbusConnectionPort> = bus.clone();
    let hub = SignalHub::new(Config::default());
    let result = ZbusMprisAdapter::new(conn, &hub).start_watching().await;
    (bus, hub, result)
}

/// `name` appearing on the bus (`new`), leaving it (`gone`) or changing owner (`switched`).
fn event(name: &str, kind: &str) -> NameOwnerChanged {
    let owner = Some(Destination::new(":1.7"));
    let (old, new) = match kind {
        "new" => (None, owner),
        "gone" => (owner, None),
        _ => (owner.clone(), owner),
    };
    NameOwnerChanged::new(Destination::new(name), old, new)
}

/// The sorted player keys and the active player of `state`.
fn summary(state: &MprisState) -> (Vec<String>, Option<PlayerName>) {
    let mut keys: Vec<String> = state.players.keys().cloned().collect();
    keys.sort();
    (keys, state.active_player.clone())
}

/// The `summary` of a state holding `keys` with `active` active.
fn expect(keys: &[&str], active: Option<&str>) -> (Vec<String>, Option<PlayerName>) {
    let keys = keys.iter().map(|key| (*key).to_string()).collect();
    (keys, active.map(PlayerName::new))
}

#[test]
fn test_a_failed_name_listing_fails_before_watching_names() {
    crate::test_utils::block_on(async {
        let (bus, _events) = bus_listing(&[]);
        let bus = FakeBus { names: None, ..bus };
        let (bus, _hub, result) = started(bus).await;

        assert!(matches!(
            result,
            Err(DbusConnectionError::NotInitialized(BusType::Session))
        ));
        assert!(bus.name_watches.lock().unwrap().is_empty());
    });
}

#[test]
fn test_only_mpris_names_already_on_the_bus_are_loaded() {
    crate::test_utils::block_on(async {
        let names = ["org.freedesktop.Notifications", &mpris_name("vlc")];
        let (bus, _events) = bus_listing(&names);
        let (bus, hub, result) = started(bus).await;

        assert!(result.is_ok());
        assert_eq!(*bus.reads.lock().unwrap(), [player_read("vlc")]);
        assert_eq!(*bus.name_watches.lock().unwrap(), [BusType::Session]);
        assert_eq!(
            summary(&hub.mpris_rx().borrow()),
            expect(&["vlc"], Some("vlc"))
        );
    });
}

#[test]
fn test_a_failed_name_subscription_fails_after_loading_the_players() {
    crate::test_utils::block_on(async {
        let (bus, _events) = bus_listing(&[&mpris_name("vlc")]);
        let bus = FakeBus {
            name_changes: std::sync::Mutex::new(None),
            ..bus
        };
        let (_bus, hub, result) = started(bus).await;

        assert!(matches!(
            result,
            Err(DbusConnectionError::NotInitialized(BusType::Session))
        ));
        assert_eq!(
            summary(&hub.mpris_rx().borrow()),
            expect(&["vlc"], Some("vlc"))
        );
    });
}

#[test]
fn test_new_mpris_names_are_added_and_other_names_ignored() {
    crate::test_utils::block_on(async {
        let (bus, events) = bus_listing(&[]);
        let (bus, hub, _result) = started(bus).await;
        let mut rx = hub.mpris_rx();

        events
            .send(event("org.freedesktop.Notifications", "new"))
            .unwrap();
        events.send(event(&mpris_name("vlc"), "switched")).unwrap();
        events.send(event(&mpris_name("mpv"), "new")).unwrap();

        assert!(reaches(&mut rx, |state| !state.players.is_empty()).await);
        assert_eq!(summary(&rx.borrow()), expect(&["mpv"], Some("mpv")));
        assert_eq!(*bus.reads.lock().unwrap(), [player_read("mpv")]);
    });
}

#[test]
fn test_a_gone_active_player_hands_over_to_the_remaining_one() {
    crate::test_utils::block_on(async {
        let (bus, events) = bus_listing(&[&mpris_name("vlc"), &mpris_name("mpv")]);
        let (_bus, hub, _result) = started(bus).await;
        let mut rx = hub.mpris_rx();

        events.send(event(&mpris_name("vlc"), "gone")).unwrap();

        assert!(reaches(&mut rx, |state| state.players.len() == 1).await);
        assert_eq!(summary(&rx.borrow()), expect(&["mpv"], Some("mpv")));
    });
}

#[test]
fn test_a_gone_inactive_player_keeps_the_active_one() {
    crate::test_utils::block_on(async {
        let (bus, events) = bus_listing(&[&mpris_name("vlc"), &mpris_name("mpv")]);
        let (_bus, hub, _result) = started(bus).await;
        let elsewhere = Some(PlayerName::new("elsewhere"));
        hub.mpris_tx()
            .send_modify(|state| state.active_player.clone_from(&elsewhere));
        let mut rx = hub.mpris_rx();

        events.send(event(&mpris_name("mpv"), "gone")).unwrap();

        assert!(reaches(&mut rx, |state| state.players.len() == 1).await);
        assert_eq!(summary(&rx.borrow()), expect(&["vlc"], Some("elsewhere")));
    });
}

#[test]
fn test_the_last_gone_player_leaves_no_active_player() {
    crate::test_utils::block_on(async {
        let (bus, events) = bus_listing(&[&mpris_name("vlc")]);
        let (_bus, hub, _result) = started(bus).await;
        let mut rx = hub.mpris_rx();

        events
            .send(event("org.freedesktop.Notifications", "gone"))
            .unwrap();
        events.send(event(&mpris_name("vlc"), "gone")).unwrap();

        assert!(reaches(&mut rx, |state| state.players.is_empty()).await);
        assert_eq!(summary(&rx.borrow()), expect(&[], None));
    });
}
