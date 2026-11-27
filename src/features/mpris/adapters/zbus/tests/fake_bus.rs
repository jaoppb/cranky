//! A scripted session bus for the MPRIS adapter tests, and the helpers they share.
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::{mpsc, watch};
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::features::mpris::domain::MprisState;
use crate::shared::dbus::domain::{
    BusObject, BusType, DBusSubscription, DBusValue, Destination, Interface, NameChangedStream,
    NameOwnerChanged, PropertiesMap, PropertyChangedStream, PropertyName, QualifiedMember,
    SignalStream,
};
use crate::shared::dbus::ports::{DbusConnectionError, DbusConnectionPort};

/// One `PropertiesChanged` signal: the interface and the properties it carries.
pub(super) type Change = (Interface, PropertiesMap);

/// A bus that answers from canned players and records every request it receives.
#[derive(Default)]
pub(super) struct FakeBus {
    /// What `ListNames` returns; `None` makes it fail.
    pub(super) names: Option<Vec<Destination>>,
    /// The `GetAll` answer per bus name; a name missing here makes it fail.
    pub(super) players: HashMap<String, PropertiesMap>,
    /// The `PropertiesChanged` stream per bus name; a name missing here makes it fail.
    pub(super) changes: Mutex<HashMap<String, mpsc::UnboundedReceiver<Change>>>,
    /// The `NameOwnerChanged` stream; `None` makes the subscription fail.
    pub(super) name_changes: Mutex<Option<mpsc::UnboundedReceiver<NameOwnerChanged>>>,
    /// Every `GetAll`, as `bus destination path interface`.
    pub(super) reads: Mutex<Vec<String>>,
    /// Every `PropertiesChanged` subscription, as `bus destination path`.
    pub(super) watched: Mutex<Vec<String>>,
    /// Every `NameOwnerChanged` subscription, by bus.
    pub(super) name_watches: Mutex<Vec<BusType>>,
}

impl FakeBus {
    /// Adds the player `name` answering `GetAll` with `props`, and returns the sender that
    /// feeds its `PropertiesChanged` stream.
    pub(super) fn with_player(
        &mut self,
        name: &str,
        props: PropertiesMap,
    ) -> mpsc::UnboundedSender<Change> {
        let (sender, receiver) = mpsc::unbounded_channel();
        self.players.insert(name.to_string(), props);
        self.changes
            .lock()
            .unwrap()
            .insert(name.to_string(), receiver);
        sender
    }

    /// Opens the `NameOwnerChanged` stream and returns the sender that feeds it.
    pub(super) fn with_name_changes(&mut self) -> mpsc::UnboundedSender<NameOwnerChanged> {
        let (sender, receiver) = mpsc::unbounded_channel();
        self.name_changes = Mutex::new(Some(receiver));
        sender
    }
}

/// `object` as `bus destination path`, the way the fake records it.
fn describe(object: &BusObject) -> String {
    let (bus, dest, path) = (object.bus(), object.destination(), object.path());
    format!("{bus:?} {} {}", dest.as_str(), path.as_str())
}

#[async_trait]
impl DbusConnectionPort for FakeBus {
    async fn call_method(
        &self,
        object: &BusObject,
        _method: &QualifiedMember,
    ) -> Result<DBusValue, DbusConnectionError> {
        Err(DbusConnectionError::NotInitialized(object.bus()))
    }

    async fn get_all_properties(
        &self,
        object: &BusObject,
        interface: &Interface,
    ) -> Result<PropertiesMap, DbusConnectionError> {
        let read = format!("{} {}", describe(object), interface.as_str());
        self.reads.lock().unwrap().push(read);
        let player = self.players.get(object.destination().as_str()).cloned();
        player.ok_or_else(|| DbusConnectionError::MethodCallFailed("no such player".into()))
    }

    async fn subscribe_properties_changed(
        &self,
        object: &BusObject,
    ) -> Result<PropertyChangedStream, DbusConnectionError> {
        self.watched.lock().unwrap().push(describe(object));
        let name = object.destination().as_str();
        let receiver = self.changes.lock().unwrap().remove(name);
        let receiver =
            receiver.ok_or_else(|| DbusConnectionError::SubscriptionFailed("no".into()))?;
        Ok(Box::pin(UnboundedReceiverStream::new(receiver)))
    }

    async fn list_names(&self, bus: BusType) -> Result<Vec<Destination>, DbusConnectionError> {
        self.names
            .clone()
            .ok_or(DbusConnectionError::NotInitialized(bus))
    }

    async fn subscribe_name_changes(
        &self,
        bus: BusType,
    ) -> Result<NameChangedStream, DbusConnectionError> {
        self.name_watches.lock().unwrap().push(bus);
        let receiver = self.name_changes.lock().unwrap().take();
        let receiver = receiver.ok_or(DbusConnectionError::NotInitialized(bus))?;
        Ok(Box::pin(UnboundedReceiverStream::new(receiver)))
    }

    async fn subscribe_signal(
        &self,
        sub: DBusSubscription,
    ) -> Result<SignalStream, DbusConnectionError> {
        Err(DbusConnectionError::SubscriptionFailed(format!("{sub:?}")))
    }
}

/// A property map holding `entries`.
pub(super) fn props(entries: Vec<(&str, DBusValue)>) -> PropertiesMap {
    let map = entries
        .into_iter()
        .map(|(name, value)| (PropertyName::new(name), value));
    PropertiesMap::new(map.collect())
}

/// A `Metadata` dictionary holding `entries`.
pub(super) fn metadata(entries: Vec<(&str, DBusValue)>) -> DBusValue {
    let map = entries
        .into_iter()
        .map(|(key, value)| (key.to_string(), value));
    DBusValue::Dict(map.collect())
}

/// A string value.
pub(super) fn text(value: &str) -> DBusValue {
    DBusValue::String(value.to_string())
}

/// The MPRIS bus name of the player `short`.
pub(super) fn mpris_name(short: &str) -> String {
    format!("org.mpris.MediaPlayer2.{short}")
}

/// The `GetAll` request the adapter makes for the player `short`.
pub(super) fn player_read(short: &str) -> String {
    format!(
        "Session {} /org/mpris/MediaPlayer2 org.mpris.MediaPlayer2.Player",
        mpris_name(short)
    )
}

/// The `PropertiesChanged` subscription the adapter makes for the player `short`.
pub(super) fn player_watch(short: &str) -> String {
    format!("Session {} /org/mpris/MediaPlayer2", mpris_name(short))
}

/// Waits up to five seconds for the published MPRIS state to satisfy `wanted`.
pub(super) async fn reaches(
    rx: &mut watch::Receiver<MprisState>,
    wanted: impl Fn(&MprisState) -> bool + Sync,
) -> bool {
    let wait = async {
        while !wanted(&rx.borrow_and_update()) {
            rx.changed().await.unwrap();
        }
    };
    tokio::time::timeout(Duration::from_secs(5), wait)
        .await
        .is_ok()
}

/// Lets every spawned task run until it waits on something, on the single-threaded test
/// runtime.
pub(super) async fn let_tasks_run() {
    for _ in 0..20 {
        tokio::task::yield_now().await;
    }
}
