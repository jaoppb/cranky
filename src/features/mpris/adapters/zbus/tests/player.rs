//! How `add_player` reads, publishes and then follows one MPRIS player.
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;

use super::fake_bus::{
    FakeBus, let_tasks_run, metadata, mpris_name, player_read, player_watch, props, reaches, text,
};
use crate::features::mpris::adapters::zbus::player::add_player;
use crate::features::mpris::domain::{
    MprisState, PlaybackStatus, PlayerName, PlayerState, TrackName,
};
use crate::shared::dbus::domain::{Destination, Interface, PropertiesMap};
use crate::shared::dbus::ports::DbusConnectionPort;

/// The `GetAll` answer of a player playing `title`.
fn playing(title: &str) -> PropertiesMap {
    let meta = metadata(vec![("xesam:title", text(title))]);
    props(vec![
        ("PlaybackStatus", text("Playing")),
        ("Metadata", meta),
    ])
}

/// The state `key` holds after reading `playing(title)`.
fn playing_state(key: &str, title: &str) -> PlayerState {
    let mut state = PlayerState::new(PlayerName::new(key));
    state.set_status(PlaybackStatus::Playing);
    state.set_track_name(Some(TrackName::new(title)));
    state
}

/// The bus and the MPRIS channel after `added` ran, with a receiver kept so sends land.
struct Added {
    bus: Arc<FakeBus>,
    tx: watch::Sender<MprisState>,
    rx: watch::Receiver<MprisState>,
}

/// Runs `add_player` for each bus name in `names` on `bus`, on a fresh MPRIS channel.
async fn added(bus: FakeBus, names: &[&str]) -> Added {
    let bus = Arc::new(bus);
    let conn: Arc<dyn DbusConnectionPort> = bus.clone();
    let (tx, rx) = watch::channel(MprisState::default());
    for name in names {
        add_player(&conn, &tx, Destination::new(*name)).await;
    }
    let_tasks_run().await;
    Added { bus, tx, rx }
}

/// Waits up to five seconds for every property listener, each holding a clone of `bus`, to
/// end. A listener ends only once its change stream is closed and drained, so every change
/// sent before the close has then been handled.
async fn listeners_end(bus: &Arc<FakeBus>) -> bool {
    let wait = async {
        while Arc::strong_count(bus) > 1 {
            tokio::task::yield_now().await;
        }
    };
    tokio::time::timeout(Duration::from_secs(5), wait)
        .await
        .is_ok()
}

#[test]
fn test_add_player_reads_the_player_interface_and_publishes_it_as_active() {
    crate::test_utils::block_on(async {
        let mut bus = FakeBus::default();
        let _changes = bus.with_player(&mpris_name("vlc"), playing("Song"));
        let Added { bus, tx, .. } = added(bus, &[&mpris_name("vlc")]).await;

        let state = tx.borrow().clone();
        assert_eq!(
            state.players.get("vlc"),
            Some(&playing_state("vlc", "Song"))
        );
        assert_eq!(state.players.len(), 1);
        assert_eq!(state.active_player, Some(PlayerName::new("vlc")));
        assert_eq!(*bus.reads.lock().unwrap(), [player_read("vlc")]);
        assert_eq!(*bus.watched.lock().unwrap(), [player_watch("vlc")]);
    });
}

#[test]
fn test_a_second_player_does_not_take_over_the_active_one() {
    crate::test_utils::block_on(async {
        let mut bus = FakeBus::default();
        let _vlc = bus.with_player(&mpris_name("vlc"), playing("Song"));
        let _mpv = bus.with_player(&mpris_name("mpv"), playing("Clip"));
        let Added { tx, .. } = added(bus, &[&mpris_name("vlc"), &mpris_name("mpv")]).await;

        let state = tx.borrow().clone();
        assert_eq!(
            state.players.get("mpv"),
            Some(&playing_state("mpv", "Clip"))
        );
        assert_eq!(state.players.len(), 2);
        assert_eq!(state.active_player, Some(PlayerName::new("vlc")));
    });
}

/// The whole name keys the player both when it is added and when its changes arrive.
#[test]
fn test_a_name_without_the_mpris_prefix_is_kept_whole() {
    crate::test_utils::block_on(async {
        let mut bus = FakeBus::default();
        let changes = bus.with_player("org.example.Player", playing("Song"));
        let Added { tx, mut rx, .. } = added(bus, &["org.example.Player"]).await;

        let state = tx.borrow().clone();
        let mut expected = playing_state("org.example.Player", "Song");
        assert_eq!(state.players.get("org.example.Player"), Some(&expected));
        assert_eq!(
            state.active_player,
            Some(PlayerName::new("org.example.Player"))
        );

        let paused = props(vec![("PlaybackStatus", text("Paused"))]);
        let player = Interface::new("org.mpris.MediaPlayer2.Player");
        changes.send((player, paused)).unwrap();
        expected.set_status(PlaybackStatus::Paused);
        let updated =
            |state: &MprisState| state.players.get("org.example.Player") == Some(&expected);
        assert!(reaches(&mut rx, updated).await);
    });
}

#[test]
fn test_a_failed_read_publishes_nothing_and_follows_nothing() {
    crate::test_utils::block_on(async {
        let Added { bus, tx: _tx, rx } = added(FakeBus::default(), &[&mpris_name("vlc")]).await;

        assert_eq!(*rx.borrow(), MprisState::default());
        assert!(!rx.has_changed().unwrap());
        assert_eq!(*bus.reads.lock().unwrap(), [player_read("vlc")]);
        assert!(bus.watched.lock().unwrap().is_empty());
    });
}

#[test]
fn test_a_failed_subscription_still_publishes_the_player() {
    crate::test_utils::block_on(async {
        let mut bus = FakeBus::default();
        bus.players.insert(mpris_name("vlc"), playing("Song"));
        let Added { bus, tx, .. } = added(bus, &[&mpris_name("vlc")]).await;

        assert_eq!(
            tx.borrow().players.get("vlc"),
            Some(&playing_state("vlc", "Song"))
        );
        assert_eq!(*bus.watched.lock().unwrap(), [player_watch("vlc")]);
    });
}

#[test]
fn test_player_interface_changes_update_the_player_and_others_are_ignored() {
    crate::test_utils::block_on(async {
        let mut bus = FakeBus::default();
        let changes = bus.with_player(&mpris_name("vlc"), playing("Song"));
        let Added { tx, mut rx, .. } = added(bus, &[&mpris_name("vlc")]).await;

        let other = Interface::new("org.mpris.MediaPlayer2");
        changes.send((other, playing("Ignored"))).unwrap();
        let player = Interface::new("org.mpris.MediaPlayer2.Player");
        changes
            .send((player, props(vec![("PlaybackStatus", text("Paused"))])))
            .unwrap();

        let mut expected = playing_state("vlc", "Song");
        expected.set_status(PlaybackStatus::Paused);
        assert!(reaches(&mut rx, |state| state.players.get("vlc") == Some(&expected)).await);
        assert_eq!(tx.borrow().active_player, Some(PlayerName::new("vlc")));
    });
}

#[test]
fn test_changes_for_a_player_no_longer_published_are_dropped() {
    crate::test_utils::block_on(async {
        let mut bus = FakeBus::default();
        let changes = bus.with_player(&mpris_name("vlc"), playing("Song"));
        let Added { bus, tx, rx: _rx } = added(bus, &[&mpris_name("vlc")]).await;
        tx.send(MprisState::default()).unwrap();

        let player = Interface::new("org.mpris.MediaPlayer2.Player");
        changes.send((player, playing("Back"))).unwrap();
        drop(changes);

        assert!(listeners_end(&bus).await);
        assert_eq!(*tx.borrow(), MprisState::default());
    });
}
