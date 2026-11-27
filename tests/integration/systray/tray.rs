//! A started tray adapter with one fake item registered on it.

use super::item::{Call, ItemService};
use crate::support::{Context, Failure, Outcome, ensure, ensure_eq, within};
use cranky::features::systray::adapters::SniAdapter;
use cranky::features::systray::domain::{
    SystrayActionCall, SystrayActionName, SystrayId, SystrayState,
};
use cranky::features::systray::ports::SniPort;
use cranky::shared::config::domain::Config;
use cranky::shared::events::signals::SignalHub;
use cranky::shared::primitives::geometry::Position;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio_stream::StreamExt;

const WATCHER: &str = "org.kde.StatusNotifierWatcher";

/// How often a signal is emitted again while the tray has not reacted.
const RE_EMIT: Duration = Duration::from_millis(50);

pub(crate) struct Tray {
    adapter: SniAdapter,
    state: watch::Receiver<SystrayState>,
    item: ItemService,
    id: SystrayId,
}

impl Tray {
    /// Starts the adapter, registers a fake item and waits for the tray to
    /// track it.
    pub(crate) async fn with_item() -> Outcome<Self> {
        Self::tracking(ItemService::start().await?).await
    }

    /// The same, with an item that sets `ItemIsMenu`.
    pub(crate) async fn with_menu_item() -> Outcome<Self> {
        let item = ItemService::start().await?;
        item.mark_as_menu().await?;
        Self::tracking(item).await
    }

    async fn tracking(item: ItemService) -> Outcome<Self> {
        let hub = Arc::new(SignalHub::new(Config::default()));
        let adapter = start_adapter(&hub).await?;
        let id = SystrayId::new(item.tray_id()?);
        item.register().await?;
        let mut state = hub.systray_rx();
        within("the item to be tracked", tracked(&mut state, &id, |_| true)).await??;
        Ok(Self {
            adapter,
            state,
            item,
            id,
        })
    }

    /// The tracked item as the scripts see it now.
    pub(crate) fn item(&self) -> Outcome<Value> {
        item_json(&self.state.borrow(), &self.id).context("the tray no longer tracks the item")
    }

    /// Emits a signal again and again until the tracked item satisfies
    /// `accept`, with no deadline of its own: callers bound it with `within`.
    /// The tray publishes an item before it has subscribed to the item's
    /// signals (#42), so the first emissions can be lost.
    pub(crate) async fn emit_until<'a, F>(
        &'a mut self,
        emit: impl Fn(&'a ItemService) -> F + Send + Sync,
        accept: impl Fn(&Value) -> bool + Send + Sync,
    ) -> Outcome
    where
        F: Future<Output = Outcome> + Send,
    {
        let (item, state, id) = (&self.item, &mut self.state, &self.id);
        loop {
            emit(item).await?;
            if let Ok(seen) = tokio::time::timeout(RE_EMIT, tracked(state, id, &accept)).await {
                return seen.map(|_| ());
            }
        }
    }

    pub(crate) async fn trigger(
        &self,
        action: SystrayActionName,
        position: Option<Position>,
    ) -> Outcome {
        self.adapter
            .trigger_action(&self.id, &SystrayActionCall::new(action, position))
            .await
            .context("trigger_action")
    }

    /// The next method the tray called on the item must be `expected`.
    pub(crate) async fn expect_call(&mut self, expected: Call) -> Outcome {
        let call = within("the item to be called", self.item.next_call())
            .await?
            .context("the item call channel closed")?;
        ensure_eq("call received by the item", &call, &expected)
    }

    /// Each of `expected`, in order, must be the next method the tray
    /// called, and no other call may follow: `trigger` returns once the item
    /// answered every call, so a call still to come would already be here.
    pub(crate) async fn expect_only_calls(&mut self, expected: Vec<Call>) -> Outcome {
        for call in expected {
            self.expect_call(call).await?;
        }
        ensure_eq(
            "call after the expected ones",
            &self.item.pending_call(),
            &None,
        )
    }

    /// Makes the item reject each of `methods` from now on.
    pub(crate) async fn reject(&self, methods: &[&'static str]) -> Outcome {
        self.item.reject(methods).await
    }

    /// Takes the item off the bus, as an application that quit, and waits
    /// until the tray no longer tracks it.
    pub(crate) async fn until_item_left(self) -> Outcome {
        let Self {
            adapter: _adapter,
            mut state,
            item,
            id,
        } = self;
        item.disconnect().await?;
        within(
            "the item to be removed",
            state.wait_for(|state| !state.items().contains_key(&id)),
        )
        .await?
        .map(|_| ())
        .context("systray state channel")
    }
}

fn item_json(state: &SystrayState, id: &SystrayId) -> Option<Value> {
    state
        .items()
        .get(id)
        .and_then(|item| serde_json::to_value(item).ok())
}

/// The item `id` as the scripts see it, once the tray tracks it and
/// `accept` holds for it.
async fn tracked(
    state: &mut watch::Receiver<SystrayState>,
    id: &SystrayId,
    accept: impl Fn(&Value) -> bool + Send + Sync,
) -> Outcome<Value> {
    let state = state
        .wait_for(|state| item_json(state, id).is_some_and(|item| accept(&item)))
        .await
        .context("systray state channel")?;
    item_json(&state, id).context("the tracked item vanished")
}

/// Registers a host with the watcher, as a second tray would, and reads back
/// whether the watcher reports a registered host.
pub(crate) async fn host_registered() -> Outcome<bool> {
    let connection = zbus::Connection::session()
        .await
        .context("host connection")?;
    connection
        .call_method(
            Some(WATCHER),
            "/StatusNotifierWatcher",
            Some(WATCHER),
            "RegisterStatusNotifierHost",
            &("org.kde.StatusNotifierHost-test",),
        )
        .await
        .context("RegisterStatusNotifierHost")?;
    is_host_registered(&connection).await
}

/// Whether the watcher reports a registered host.
async fn is_host_registered(connection: &zbus::Connection) -> Outcome<bool> {
    let properties = zbus::fdo::PropertiesProxy::builder(connection)
        .destination(WATCHER)
        .path("/StatusNotifierWatcher")
        .build()
        .await
        .context("watcher properties proxy")?;
    let interface = zbus::names::InterfaceName::try_from(WATCHER).context("watcher interface")?;
    let value = properties
        .get(interface, "IsStatusNotifierHostRegistered")
        .await
        .context("IsStatusNotifierHostRegistered")?;
    bool::try_from(value).context("IsStatusNotifierHostRegistered as a bool")
}

/// Returns once `name` has no owner: at once when it has none now, otherwise
/// when `changes` reports that its owner left.
async fn wait_until_unowned(
    bus: &zbus::fdo::DBusProxy<'_>,
    changes: &mut zbus::fdo::NameOwnerChangedStream,
    name: &zbus::names::BusName<'_>,
) -> Outcome {
    if bus
        .name_has_owner(name.clone())
        .await
        .context("NameHasOwner")?
    {
        within(
            "the previous watcher to leave the bus",
            owner_left(changes, name),
        )
        .await??;
    }
    Ok(())
}

/// Waits until `changes` reports that `name` lost its owner.
async fn owner_left(
    changes: &mut zbus::fdo::NameOwnerChangedStream,
    name: &zbus::names::BusName<'_>,
) -> Outcome {
    loop {
        let change = changes
            .next()
            .await
            .context("the name change stream ended")?;
        if let Ok(args) = change.args()
            && args.name() == name
            && args.new_owner().is_none()
        {
            return Ok::<_, Failure>(());
        }
    }
}

/// Starts the adapter once the watcher name is free. The previous case's
/// process has exited, but the daemon learns that on its own schedule, and
/// an adapter that finds the name taken silently runs without a watcher.
pub(crate) async fn start_adapter(hub: &Arc<SignalHub>) -> Outcome<SniAdapter> {
    let connection = zbus::Connection::session()
        .await
        .context("observer connection")?;
    let bus = zbus::fdo::DBusProxy::new(&connection)
        .await
        .context("bus proxy")?;
    let mut changes = bus
        .receive_name_owner_changed()
        .await
        .context("subscribe to name changes")?;
    let name = zbus::names::BusName::try_from(WATCHER).context("watcher name")?;
    wait_until_unowned(&bus, &mut changes, &name).await?;

    let mut adapter = SniAdapter::new(Arc::clone(hub));
    adapter.start().await.context("start the SNI adapter")?;
    let owned = bus.name_has_owner(name).await.context("NameHasOwner")?;
    ensure(
        || owned,
        || format!("{WATCHER} has no owner after the adapter started"),
    )?;
    Ok(adapter)
}
