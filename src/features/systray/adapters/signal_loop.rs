use super::sni_event::SniEvent;
use super::sni_proxy::{
    NewAttentionIconStream, NewIconStream, NewIconThemePathStream, NewMenuStream,
    NewOverlayIconStream, NewStatusStream, NewTitleStream, NewToolTipStream,
    StatusNotifierItemProxy,
};
use super::tray_items::TrayItems;
use crate::features::systray::domain::SystrayId;
use std::ops::ControlFlow;
use tokio_stream::{Stream, StreamExt};

/// The first four change signals of an item, in the order they are subscribed.
struct MainSignals {
    title: NewTitleStream,
    icon: NewIconStream,
    status: NewStatusStream,
    theme_path: NewIconThemePathStream,
}

/// The last four change signals of an item, in the order they are subscribed.
struct ExtraSignals {
    attention_icon: NewAttentionIconStream,
    overlay_icon: NewOverlayIconStream,
    tool_tip: NewToolTipStream,
    menu: NewMenuStream,
}

/// The stream a subscription returned, or `None` after logging that it failed.
fn subscribed<T>(subscription: zbus::Result<T>, signal: &str, id: &str) -> Option<T> {
    let Ok(stream) = subscription else {
        tracing::error!("Failed to subscribe to {signal} for {id}");
        return None;
    };
    Some(stream)
}

impl MainSignals {
    /// Subscribes to each signal in turn, stopping at the first that fails.
    async fn subscribe(proxy: &StatusNotifierItemProxy<'static>, id: &str) -> Option<Self> {
        let title = subscribed(proxy.receive_new_title().await, "new_title", id)?;
        let icon = subscribed(proxy.receive_new_icon().await, "new_icon", id)?;
        let status = subscribed(proxy.receive_new_status().await, "new_status", id)?;
        let theme_path = proxy.receive_new_icon_theme_path().await;
        let theme_path = subscribed(theme_path, "new_icon_theme_path", id)?;
        Some(Self {
            title,
            icon,
            status,
            theme_path,
        })
    }
}

impl ExtraSignals {
    /// Subscribes to each signal in turn, stopping at the first that fails.
    async fn subscribe(proxy: &StatusNotifierItemProxy<'static>, id: &str) -> Option<Self> {
        let attention_icon = proxy.receive_new_attention_icon().await;
        let attention_icon = subscribed(attention_icon, "new_attention_icon", id)?;
        let overlay_icon = proxy.receive_new_overlay_icon().await;
        let overlay_icon = subscribed(overlay_icon, "new_overlay_icon", id)?;
        let tool_tip = subscribed(proxy.receive_new_tool_tip().await, "new_tool_tip", id)?;
        let menu = subscribed(proxy.receive_new_menu().await, "new_menu", id)?;
        Some(Self {
            attention_icon,
            overlay_icon,
            tool_tip,
            menu,
        })
    }
}

/// One stream of the events every signal stands for, in arrival order.
fn sni_events(
    main: MainSignals,
    extra: ExtraSignals,
) -> impl Stream<Item = SniEvent> + Unpin + Send {
    main.title
        .map(|_| SniEvent::Title)
        .merge(
            main.status.map(|sig| {
                SniEvent::Status(sig.args().map_or_default(|args| args.status().clone()))
            }),
        )
        .merge(main.icon.map(|_| SniEvent::Icon))
        .merge(main.theme_path.map(|_| SniEvent::ThemePath))
        .merge(extra.attention_icon.map(|_| SniEvent::AttentionIcon))
        .merge(extra.overlay_icon.map(|_| SniEvent::OverlayIcon))
        .merge(extra.tool_tip.map(|_| SniEvent::ToolTip))
        .merge(extra.menu.map(|_| SniEvent::Menu))
}

/// Keeps one tracked item up to date with its signals.
struct ItemFollower {
    proxy: StatusNotifierItemProxy<'static>,
    tray: TrayItems,
    id: String,
}

impl ItemFollower {
    /// Applies every event until the stream ends or the item is no longer tracked, then drops the
    /// item from the tray.
    async fn follow(self, events: impl Stream<Item = SniEvent> + Unpin + Send) {
        self.apply_all(events).await;
        log_terminated(&self.id);
        let id = SystrayId::new(&self.id);
        self.tray.remove_and_publish(&id).await;
    }

    async fn apply_all(&self, mut events: impl Stream<Item = SniEvent> + Unpin + Send) {
        while let Some(event) = events.next().await {
            if self.apply(event).await.is_break() {
                break;
            }
        }
    }

    /// Refetches what `event` announces into the tracked item and publishes it; breaks when the
    /// item is no longer tracked.
    async fn apply(&self, event: SniEvent) -> ControlFlow<()> {
        log_event(&event, &self.id);
        let current_item = {
            let lock = self.tray.items().read().await;
            lock.get(&SystrayId::new(&self.id)).cloned()
        };
        let Some(item) = current_item else {
            return ControlFlow::Break(());
        };
        let updated_item = event.apply(item, &self.proxy).await;
        let id = SystrayId::new(&self.id);
        self.tray.insert_and_publish(id, updated_item).await;
        ControlFlow::Continue(())
    }
}

fn log_subscribed(id: &str) {
    tracing::debug!("Successfully subscribed to all SNI signals for {id}");
}

fn log_event(event: &SniEvent, id: &str) {
    tracing::trace!("Received SNI event {event:?} for systray item {id}");
}

fn log_terminated(id: &str) {
    tracing::debug!("Systray item {id} loop terminated, cleaning up state");
}

pub(super) async fn run_signal_loop(
    proxy: StatusNotifierItemProxy<'static>,
    tray: TrayItems,
    id: String,
) {
    let Some(main) = MainSignals::subscribe(&proxy, &id).await else {
        return;
    };
    let Some(extra) = ExtraSignals::subscribe(&proxy, &id).await else {
        return;
    };
    log_subscribed(&id);
    let follower = ItemFollower { proxy, tray, id };
    tokio::spawn(follower.follow(sni_events(main, extra)));
}
