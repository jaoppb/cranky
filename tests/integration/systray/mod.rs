//! The `StatusNotifier` adapters (`src/features/systray/adapters/`) against the
//! private bus: watcher, fetcher, proxy signals, actions and cleanup.

pub mod item;
pub mod tray;

use crate::cases::{Case, case};
use crate::support::{Outcome, ensure_eq};
use cranky::features::systray::domain::SystrayActionName;
use cranky::shared::config::domain::Config;
use cranky::shared::events::signals::SignalHub;
use cranky::shared::primitives::geometry::Position;
use item::Call;
use serde_json::Value;
use std::sync::Arc;
use tray::Tray;

pub(crate) fn cases() -> Vec<Case> {
    vec![
        case!(systray::claims_the_watcher_name),
        case!(systray::fetches_a_registered_item),
        case!(systray::follows_title_and_status_signals),
        case!(systray::forwards_actions_to_the_item),
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
    Tray::with_item().await?.until_item_left().await
}
