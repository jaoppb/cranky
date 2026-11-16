//! The `StatusNotifier` adapters (`src/features/systray/adapters/`) against the
//! private bus: watcher, fetcher, proxy signals, actions and cleanup.

mod item;

use crate::cases::{Case, case};
use crate::support::{Context, Failure, Outcome, WAIT, ensure, ensure_eq, within};
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
        let id = self.id.clone();
        let state = within(
            what,
            self.state
                .wait_for(|state| item_json(state, &id).is_some_and(|item| accept(&item))),
        )
        .await?
        .context("systray state channel")?;
        item_json(&state, &id).context("tracked item")
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
        let id = self.id.clone();
        let (item, state) = (&self.item, &mut self.state);
        within(what, async {
            loop {
                emit(item).await?;
                let seen =
                    state.wait_for(|state| item_json(state, &id).is_some_and(|item| accept(&item)));
                if tokio::time::timeout(RE_EMIT, seen).await.is_ok() {
                    return Ok(());
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

fn item_json(state: &SystrayState, id: &SystrayId) -> Option<Value> {
    state
        .items()
        .get(id)
        .and_then(|item| serde_json::to_value(item).ok())
}

/// Starts the adapter once the watcher name is free: the adapter of the
/// previous case may still be leaving the bus.
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
        ("title", "Fake Title"),
        ("status", "Active"),
        ("category", "Communications"),
        ("item_id", "fake-item"),
        ("path", item::PATH),
    ];
    for (field, value) in expected {
        ensure_eq(field, &item[field], &Value::from(value))?;
    }
    ensure_eq("item_is_menu", &item["item_is_menu"], &Value::from(false))?;
    // The tooltip arrives as markup and is cleaned on the way in.
    ensure_eq(
        "tooltip title",
        &item["tooltip"]["title"],
        &Value::from("Tip"),
    )?;
    ensure_eq(
        "tooltip description",
        &item["tooltip"]["description"],
        &Value::from("a & b"),
    )
}

async fn follows_title_and_status_signals() -> Outcome {
    let mut tray = Tray::with_item().await?;
    tray.item.set_title("Renamed");
    tray.emit_until(
        "NewTitle to reach the tray",
        ItemService::emit_new_title,
        |item| item["title"] == "Renamed",
    )
    .await?;
    tray.emit_until(
        "NewStatus to reach the tray",
        |item| item.emit_new_status("NeedsAttention"),
        |item| item["status"] == "NeedsAttention",
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
    within(
        "the item to be removed",
        state.wait_for(|state| !state.items().contains_key(&id)),
    )
    .await?
    .map(|_| ())
    .map_err(|_| Failure::new(format!("systray state channel closed within {WAIT:?}")))
}
