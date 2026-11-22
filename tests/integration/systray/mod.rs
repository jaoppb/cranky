//! The `StatusNotifier` adapters (`src/features/systray/adapters/`) against the
//! private bus: watcher, fetcher, proxy signals, actions and cleanup.

pub mod item;
pub mod tray;

use crate::cases::{Case, case};
use crate::support::{Outcome, ensure_eq, within};
use cranky::features::systray::domain::SystrayActionName;
use cranky::shared::config::domain::Config;
use cranky::shared::events::signals::SignalHub;
use cranky::shared::primitives::geometry::Position;
use item::{Call, ItemService};
use serde_json::Value;
use std::sync::Arc;
use tray::Tray;

pub(crate) fn cases() -> Vec<Case> {
    vec![
        case!(systray::claims_the_watcher_name),
        case!(systray::fetches_a_registered_item),
        case!(systray::follows_title_and_status_signals),
        case!(systray::follows_the_menu_signal),
        case!(systray::forwards_actions_to_the_item),
        case!(systray::primary_activates_a_plain_item),
        case!(systray::primary_opens_the_menu_of_a_menu_item),
        case!(systray::reports_a_registered_host),
        case!(systray::drops_an_item_that_left_the_bus),
    ]
}

async fn claims_the_watcher_name() -> Outcome {
    let hub = Arc::new(SignalHub::new(Config::default()));
    tray::start_adapter(&hub).await.map(|_| ())
}

async fn fetches_a_registered_item() -> Outcome {
    let item = Tray::with_item().await?.item()?;
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
    within(
        "NewTitle to reach the tray",
        tray.emit_until(
            |item| item.retitle("Renamed"),
            |item| shows(item, "/title", "Renamed"),
        ),
    )
    .await??;
    within(
        "NewStatus to reach the tray",
        tray.emit_until(
            |item| item.restatus("NeedsAttention"),
            |item| shows(item, "/status", "NeedsAttention"),
        ),
    )
    .await?
}

/// `NewMenu` refetches `ItemIsMenu`, so an item that turns into a menu-only
/// item after registering is seen as one.
async fn follows_the_menu_signal() -> Outcome {
    let mut tray = Tray::with_item().await?;
    within(
        "NewMenu to reach the tray",
        tray.emit_until(ItemService::become_menu, |item| {
            item.pointer("/item_is_menu") == Some(&Value::from(true))
        }),
    )
    .await?
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

async fn primary_activates_a_plain_item() -> Outcome {
    let mut tray = Tray::with_item().await?;
    tray.trigger(SystrayActionName::Primary, Some(Position::new(7, 8)))
        .await?;
    tray.expect_call(Call::Activate(7, 8)).await
}

async fn primary_opens_the_menu_of_a_menu_item() -> Outcome {
    let mut tray = Tray::with_menu_item().await?;
    ensure_eq(
        "/item_is_menu",
        &tray.item()?.pointer("/item_is_menu").cloned(),
        &Some(Value::from(true)),
    )?;
    tray.trigger(SystrayActionName::Primary, Some(Position::new(7, 8)))
        .await?;
    tray.expect_call(Call::ContextMenu(7, 8)).await
}

async fn reports_a_registered_host() -> Outcome {
    let hub = Arc::new(SignalHub::new(Config::default()));
    let _adapter = tray::start_adapter(&hub).await?;
    ensure_eq(
        "IsStatusNotifierHostRegistered",
        &tray::host_registered().await?,
        &true,
    )
}

async fn drops_an_item_that_left_the_bus() -> Outcome {
    Tray::with_item().await?.until_item_left().await
}
