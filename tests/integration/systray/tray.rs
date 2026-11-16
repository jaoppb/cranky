//! A started tray adapter with one fake item registered on it.

use super::item::{Call, ItemService};
use crate::support::{Context, Failure, Outcome, ensure, ensure_eq, within};
use cranky::features::systray::adapters::SniAdapter;
use cranky::features::systray::domain::{SystrayActionName, SystrayId, SystrayState};
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

pub struct Tray {
    adapter: SniAdapter,
    state: watch::Receiver<SystrayState>,
    item: ItemService,
    id: SystrayId,
}

impl Tray {
    /// Starts the adapter, registers a fake item and waits for the tray to
    /// track it.
    pub async fn with_item() -> Outcome<Self> {
        let hub = Arc::new(SignalHub::new(Config::default()));
        let adapter = start_adapter(&hub).await?;
        let item = ItemService::start().await?;
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
    pub fn item(&self) -> Outcome<Value> {
        item_json(&self.state.borrow(), &self.id).context("the tray no longer tracks the item")
    }

    /// Emits a signal again and again until the tracked item satisfies
    /// `accept`. The tray publishes an item before it has subscribed to the
    /// item's signals (#42), so the first emissions can be lost.
    pub async fn emit_until<'a, F>(
        &'a mut self,
        what: &str,
        emit: impl Fn(&'a ItemService) -> F + Send + Sync,
        accept: impl Fn(&Value) -> bool + Send + Sync,
    ) -> Outcome
    where
        F: Future<Output = Outcome> + Send,
    {
        let (item, state, id) = (&self.item, &mut self.state, &self.id);
        within(what, async {
            loop {
                emit(item).await?;
                if let Ok(seen) = tokio::time::timeout(RE_EMIT, tracked(state, id, &accept)).await {
                    return seen.map(|_| ());
                }
            }
        })
        .await?
    }

    pub async fn trigger(&self, action: SystrayActionName, at: Option<Position>) -> Outcome {
        self.adapter
            .trigger_action(&self.id, &action, at)
            .await
            .context("trigger_action")
    }

    /// The next method the tray called on the item must be `expected`.
    pub async fn expect_call(&mut self, expected: Call) -> Outcome {
        let call = within("the item to be called", self.item.next_call())
            .await?
            .context("the item call channel closed")?;
        ensure_eq("call received by the item", &call, &expected)
    }

    /// Takes the item off the bus, as an application that quit, and waits
    /// until the tray no longer tracks it.
    pub async fn until_item_left(self) -> Outcome {
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

/// Starts the adapter once the watcher name is free. The previous case's
/// process has exited, but the daemon learns that on its own schedule, and
/// an adapter that finds the name taken silently runs without a watcher.
pub async fn start_adapter(hub: &Arc<SignalHub>) -> Outcome<SniAdapter> {
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
    if bus
        .name_has_owner(name.clone())
        .await
        .context("NameHasOwner")?
    {
        within("the previous watcher to leave the bus", async {
            loop {
                let change = changes
                    .next()
                    .await
                    .context("the name change stream ended")?;
                if let Ok(args) = change.args()
                    && args.name() == &name
                    && args.new_owner().is_none()
                {
                    return Ok::<_, Failure>(());
                }
            }
        })
        .await??;
    }

    let mut adapter = SniAdapter::new(Arc::clone(hub));
    adapter.start().await.context("start the SNI adapter")?;
    ensure(
        bus.name_has_owner(name).await.context("NameHasOwner")?,
        || format!("{WATCHER} has no owner after the adapter started"),
    )?;
    Ok(adapter)
}
