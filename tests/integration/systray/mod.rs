//! The `StatusNotifier` adapters (`src/features/systray/adapters/`) against the
//! private bus: watcher, fetcher, proxy signals, actions and cleanup.

pub mod item;
pub mod themes;
pub mod tray;

use crate::cases::{Case, case};
use crate::support::{Outcome, ensure, ensure_eq, within};
use cranky::features::systray::domain::SystrayActionName;
use cranky::shared::config::domain::Config;
use cranky::shared::events::signals::SignalHub;
use cranky::shared::primitives::geometry::Position;
use item::{Call, ItemService};
use serde_json::Value;
use std::sync::Arc;
use themes::{ATTENTION_ICON, MAIN_ICON, OVERLAY_ICON, Theme};
use tray::Tray;

pub(crate) fn cases() -> Vec<Case> {
    vec![
        case!(systray::claims_the_watcher_name),
        case!(systray::fetches_a_registered_item),
        case!(systray::follows_title_and_status_signals),
        case!(systray::follows_the_menu_signal),
        case!(systray::follows_the_menu_path_signal),
        case!(systray::follows_the_icon_signal),
        case!(systray::follows_the_theme_path_signal),
        case!(systray::follows_the_attention_icon_signal),
        case!(systray::follows_the_overlay_icon_signal),
        case!(systray::follows_the_tool_tip_signal),
        case!(systray::forwards_actions_to_the_item),
        case!(systray::primary_activates_a_plain_item),
        case!(systray::primary_falls_back_to_secondary_activate),
        case!(systray::primary_opens_the_menu_of_a_menu_item),
        case!(systray::primary_of_a_menu_item_has_no_fallback),
        case!(systray::context_menu_falls_back_to_secondary_activate),
        case!(systray::context_menu_falls_back_to_activate_last),
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

/// `NewMenu` refetches `Menu` too, read as the object path it is.
async fn follows_the_menu_path_signal() -> Outcome {
    let mut tray = Tray::with_item().await?;
    within(
        "NewMenu to reach the tray",
        tray.emit_until(
            |item| item.move_menu("/MenuBar"),
            |item| shows(item, "/menu_path", "/MenuBar"),
        ),
    )
    .await?
}

/// Whether the icon at `pointer` has an image, which only a lookup in the
/// right theme directory finds: no icon here has a pixmap.
fn has_image(item: &Value, pointer: &str) -> bool {
    item.pointer(&format!("{pointer}/image")).is_some()
}

/// The item's icon has no image before any signal: its name is in none of
/// the directories `IconThemePath` starts at.
fn no_image_yet(tray: &Tray) -> Outcome {
    let item = tray.item()?;
    ensure(
        || !has_image(&item, "/icon"),
        || format!("the icon has an image before any signal: {item}"),
    )
}

/// `NewIcon` refetches `IconName` and looks it up in `IconThemePath`.
async fn follows_the_icon_signal() -> Outcome {
    let mut tray = Tray::with_item().await?;
    no_image_yet(&tray)?;
    within(
        "NewIcon to reach the tray",
        tray.emit_until(
            |item| item.rename_icon(MAIN_ICON),
            |item| shows(item, "/icon/name", MAIN_ICON) && has_image(item, "/icon"),
        ),
    )
    .await?
}

/// `NewIconThemePath` refetches the icon: the item's icon only resolves in
/// the theme directory it moves to.
async fn follows_the_theme_path_signal() -> Outcome {
    let mut tray = Tray::with_item().await?;
    no_image_yet(&tray)?;
    within(
        "NewIconThemePath to reach the tray",
        tray.emit_until(
            |item| item.move_theme(Theme::Extra),
            |item| has_image(item, "/icon"),
        ),
    )
    .await?
}

/// `NewAttentionIcon` looks the attention icon up in
/// `AttentionIconThemePath`, the only directory that holds it.
async fn follows_the_attention_icon_signal() -> Outcome {
    let mut tray = Tray::with_item().await?;
    within(
        "NewAttentionIcon to reach the tray",
        tray.emit_until(
            |item| item.rename_attention_icon(ATTENTION_ICON),
            |item| {
                shows(item, "/attention_icon/name", ATTENTION_ICON)
                    && has_image(item, "/attention_icon")
            },
        ),
    )
    .await?
}

/// `NewOverlayIcon` looks the overlay icon up in `IconThemePath`, the only
/// directory that holds it.
async fn follows_the_overlay_icon_signal() -> Outcome {
    let mut tray = Tray::with_item().await?;
    within(
        "NewOverlayIcon to reach the tray",
        tray.emit_until(
            |item| item.rename_overlay_icon(OVERLAY_ICON),
            |item| {
                shows(item, "/overlay_icon/name", OVERLAY_ICON) && has_image(item, "/overlay_icon")
            },
        ),
    )
    .await?
}

/// `NewToolTip` refetches `ToolTip`, cleaned as on the first fetch.
async fn follows_the_tool_tip_signal() -> Outcome {
    let mut tray = Tray::with_item().await?;
    within(
        "NewToolTip to reach the tray",
        tray.emit_until(
            |item| item.retip("<i>New tip</i>"),
            |item| shows(item, "/tooltip/title", "New tip"),
        ),
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

/// A rejected `Activate` falls back to `SecondaryActivate`, and nothing
/// follows when that is rejected too.
async fn primary_falls_back_to_secondary_activate() -> Outcome {
    let mut tray = Tray::with_item().await?;
    tray.reject(&["Activate", "SecondaryActivate"]).await?;
    tray.trigger(SystrayActionName::Primary, Some(Position::new(7, 8)))
        .await?;
    let expected = vec![Call::Activate(7, 8), Call::SecondaryActivate(7, 8)];
    tray.expect_only_calls(expected).await
}

/// The menu of a menu-only item has no fallback when it is rejected.
async fn primary_of_a_menu_item_has_no_fallback() -> Outcome {
    let mut tray = Tray::with_menu_item().await?;
    tray.reject(&["ContextMenu", "SecondaryActivate", "Activate"])
        .await?;
    tray.trigger(SystrayActionName::Primary, Some(Position::new(7, 8)))
        .await?;
    tray.expect_only_calls(vec![Call::ContextMenu(7, 8)]).await
}

/// A rejected `ContextMenu` falls back to `SecondaryActivate`, and stops
/// there when it is accepted.
async fn context_menu_falls_back_to_secondary_activate() -> Outcome {
    let mut tray = Tray::with_item().await?;
    tray.reject(&["ContextMenu"]).await?;
    tray.trigger(SystrayActionName::ContextMenu, Some(Position::new(5, 6)))
        .await?;
    let expected = vec![Call::ContextMenu(5, 6), Call::SecondaryActivate(5, 6)];
    tray.expect_only_calls(expected).await
}

/// When `SecondaryActivate` is rejected too, `Activate` comes last, and
/// nothing follows it even when it is rejected as well.
async fn context_menu_falls_back_to_activate_last() -> Outcome {
    let mut tray = Tray::with_item().await?;
    tray.reject(&["ContextMenu", "SecondaryActivate", "Activate"])
        .await?;
    tray.trigger(SystrayActionName::ContextMenu, Some(Position::new(5, 6)))
        .await?;
    let expected = vec![
        Call::ContextMenu(5, 6),
        Call::SecondaryActivate(5, 6),
        Call::Activate(5, 6),
    ];
    tray.expect_only_calls(expected).await
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
