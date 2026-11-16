//! The `StatusNotifier` adapters (`src/features/systray/adapters/`) against the
//! private bus: watcher, fetcher, proxy signals, actions and cleanup.

mod item;

use crate::cases::{Case, case};
use crate::support::{Context, Outcome, ensure, ensure_eq, within};
use cranky::features::systray::adapters::SniAdapter;
use cranky::features::systray::domain::{SystrayActionName, SystrayId, SystrayState};
use cranky::features::systray::ports::SniPort;
use cranky::shared::config::domain::Config;
use cranky::shared::events::signals::SignalHub;
use cranky::shared::primitives::geometry::Position;
use item::{Call, ItemService};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio_stream::StreamExt;

const WATCHER: &str = "org.kde.StatusNotifierWatcher";

/// How often a signal is emitted again while the tray has not reacted.
const RE_EMIT: Duration = Duration::from_millis(50);

pub fn cases() -> Vec<Case> {
    vec![
        case!(systray::claims_the_watcher_name),
        case!(systray::fetches_a_registered_item),
        case!(systray::follows_title_and_status_signals),
        case!(systray::forwards_actions_to_the_item),
        case!(systray::drops_an_item_that_left_the_bus),
    ]
}

/// A started tray with one registered fake item.
struct Tray {
    adapter: SniAdapter,
    state: watch::Receiver<SystrayState>,
    item: ItemService,
    id: SystrayId,
}

impl Tray {
    async fn with_item() -> Outcome<Self> {
        let hub = Arc::new(SignalHub::new(Config::default()));
        let adapter = start_adapter(&hub).await?;
        let item = ItemService::start().await?;
        let id = SystrayId::new(item.tray_id()?);
        item.register().await?;
        let mut tray = Self {
            adapter,
            state: hub.systray_rx(),
            item,
            id,
        };
        tray.until("the item to be tracked", |_| true).await?;
        Ok(tray)
    }

    /// The tracked item as the scripts see it, once `accept` holds for it.
    async fn until(
        &mut self,
        what: &str,
        accept: impl Fn(&Value) -> bool + Send + Sync,
    ) -> Outcome<Value> {
        within(what, tracked(&mut self.state, &self.id, accept)).await?
    }

    /// Emits a signal again and again until the tracked item satisfies
    /// `accept`. The tray publishes an item before it has subscribed to the
    /// item's signals (#42), so the first emissions can be lost.
    async fn emit_until<'a, F>(
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

    async fn trigger(&self, action: SystrayActionName, at: Option<Position>) -> Outcome {
        self.adapter
            .trigger_action(&self.id, &action, at)
            .await
            .context("trigger_action")
    }

    async fn expect_call(&mut self, expected: Call) -> Outcome {
        let call = within("the item to be called", self.item.next_call())
            .await?
            .context("item call channel")?;
        ensure_eq("call received by the item", &call, &expected)
    }
}

/// The item `id` as the scripts see it, once the tray tracks it and
/// `accept` holds for it.
async fn tracked(
    state: &mut watch::Receiver<SystrayState>,
    id: &SystrayId,
    accept: impl Fn(&Value) -> bool + Send + Sync,
) -> Outcome<Value> {
    let json = |state: &SystrayState| {
        state
            .items()
            .get(id)
            .and_then(|item| serde_json::to_value(item).ok())
    };
    let state = state
        .wait_for(|state| json(state).is_some_and(|item| accept(&item)))
        .await
        .context("systray state channel")?;
    json(&state).context("tracked item")
}

/// Waits until the tray no longer tracks the item `id`.
async fn until_gone(state: &mut watch::Receiver<SystrayState>, id: &SystrayId) -> Outcome {
    within(
        "the item to be removed",
        state.wait_for(|state| !state.items().contains_key(id)),
    )
    .await?
    .map(|_| ())
    .context("systray state channel")
}

/// Starts the adapter once the watcher name is free. The previous case's
/// process has exited, but the daemon learns that on its own schedule, and
/// an adapter that finds the name taken silently runs without a watcher.
async fn start_adapter(hub: &Arc<SignalHub>) -> Outcome<SniAdapter> {
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
            while let Some(change) = changes.next().await {
                if let Ok(args) = change.args()
                    && args.name() == &name
                    && args.new_owner().is_none()
                {
                    break;
                }
            }
        })
        .await?;
    }

    let mut adapter = SniAdapter::new(Arc::clone(hub));
    adapter.start().await.context("start the SNI adapter")?;
    ensure(
        bus.name_has_owner(name).await.context("NameHasOwner")?,
        || format!("{WATCHER} has no owner after the adapter started"),
    )?;
    Ok(adapter)
}

async fn claims_the_watcher_name() -> Outcome {
    let hub = Arc::new(SignalHub::new(Config::default()));
    start_adapter(&hub).await.map(|_| ())
}

async fn fetches_a_registered_item() -> Outcome {
    let mut tray = Tray::with_item().await?;
    let item = tray.until("the fetched item", |_| true).await?;
    let expected = [
        ("/title", "Fake Title"),
        ("/status", "Active"),
        ("/category", "Communications"),
        ("/item_id", "fake-item"),
        ("/path", item::PATH),
        // The tooltip arrives as markup and is cleaned on the way in.
        ("/tooltip/title", "Tip"),
        ("/tooltip/description", "a & b"),
    ];
    for (pointer, value) in expected {
        ensure_eq(pointer, &item.pointer(pointer), &Some(&Value::from(value)))?;
    }
    ensure_eq(
        "/item_is_menu",
        &item.pointer("/item_is_menu"),
        &Some(&Value::from(false)),
    )
}

/// Whether the item, as serialised for the scripts, holds `expected` at the
/// JSON `pointer`.
fn shows(item: &Value, pointer: &str, expected: &str) -> bool {
    item.pointer(pointer).and_then(Value::as_str) == Some(expected)
}

async fn follows_title_and_status_signals() -> Outcome {
    let mut tray = Tray::with_item().await?;
    tray.emit_until(
        "NewTitle to reach the tray",
        |item| item.retitle("Renamed"),
        |item| shows(item, "/title", "Renamed"),
    )
    .await?;
    tray.emit_until(
        "NewStatus to reach the tray",
        |item| item.restatus("NeedsAttention"),
        |item| shows(item, "/status", "NeedsAttention"),
    )
    .await
}

async fn forwards_actions_to_the_item() -> Outcome {
    let mut tray = Tray::with_item().await?;
    tray.trigger(SystrayActionName::Activate, Some(Position::new(3, 4)))
        .await?;
    tray.expect_call(Call::Activate(3, 4)).await?;
    tray.trigger(SystrayActionName::ContextMenu, Some(Position::new(5, 6)))
        .await?;
    tray.expect_call(Call::ContextMenu(5, 6)).await?;
    tray.trigger(SystrayActionName::ScrollUp, None).await?;
    tray.expect_call(Call::Scroll(-1, "vertical".to_string()))
        .await
}

async fn drops_an_item_that_left_the_bus() -> Outcome {
    let Tray {
        adapter: _adapter,
        mut state,
        item,
        id,
    } = Tray::with_item().await?;
    item.disconnect().await?;
    until_gone(&mut state, &id).await
}
