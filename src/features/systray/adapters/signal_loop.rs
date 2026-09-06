use super::sni_event::SniEvent;
use super::sni_proxy::StatusNotifierItemProxy;
use super::watcher::Watcher;
use crate::features::systray::domain::{SystrayId, SystrayItem};
use crate::shared::events::signals::SignalHub;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn run_signal_loop(
    proxy: StatusNotifierItemProxy<'static>,
    items: Arc<RwLock<BTreeMap<SystrayId, SystrayItem>>>,
    hub: Arc<SignalHub>,
    id: String,
) {
    let Ok(new_title) = proxy.receive_new_title().await else {
        tracing::error!("Failed to subscribe to new_title for {id}");
        return;
    };
    let Ok(new_icon) = proxy.receive_new_icon().await else {
        tracing::error!("Failed to subscribe to new_icon for {id}");
        return;
    };
    let Ok(new_status) = proxy.receive_new_status().await else {
        tracing::error!("Failed to subscribe to new_status for {id}");
        return;
    };
    let Ok(new_path) = proxy.receive_new_icon_theme_path().await else {
        tracing::error!("Failed to subscribe to new_icon_theme_path for {id}");
        return;
    };
    let Ok(new_attention_icon) = proxy.receive_new_attention_icon().await else {
        tracing::error!("Failed to subscribe to new_attention_icon for {id}");
        return;
    };
    let Ok(new_overlay_icon) = proxy.receive_new_overlay_icon().await else {
        tracing::error!("Failed to subscribe to new_overlay_icon for {id}");
        return;
    };
    let Ok(new_tool_tip) = proxy.receive_new_tool_tip().await else {
        tracing::error!("Failed to subscribe to new_tool_tip for {id}");
        return;
    };
    let Ok(new_menu) = proxy.receive_new_menu().await else {
        tracing::error!("Failed to subscribe to new_menu for {id}");
        return;
    };

    tracing::debug!("Successfully subscribed to all SNI signals for {id}");

    tokio::spawn(async move {
        use tokio_stream::StreamExt;
        let mut events = new_title
            .map(|_| SniEvent::Title)
            .merge(new_status.map(|sig| {
                SniEvent::Status(sig.args().map(|a| a.status().clone()).unwrap_or_default())
            }))
            .merge(new_icon.map(|_| SniEvent::Icon))
            .merge(new_path.map(|_| SniEvent::ThemePath))
            .merge(new_attention_icon.map(|_| SniEvent::AttentionIcon))
            .merge(new_overlay_icon.map(|_| SniEvent::OverlayIcon))
            .merge(new_tool_tip.map(|_| SniEvent::ToolTip))
            .merge(new_menu.map(|_| SniEvent::Menu));

        while let Some(event) = events.next().await {
            tracing::trace!("Received SNI event {event:?} for systray item {id}");
            let current_item = {
                let lock = items.read().await;
                lock.get(&SystrayId::new(&id)).cloned()
            };

            let Some(item) = current_item else {
                break;
            };

            let updated_item = event.apply(item, &proxy).await;

            {
                let mut lock = items.write().await;
                lock.insert(SystrayId::new(&id), updated_item);
            }
            Watcher::publish_state(&items, &hub).await;
        }

        tracing::debug!("Systray item {id} loop terminated, cleaning up state");
        {
            let mut lock = items.write().await;
            lock.remove(&SystrayId::new(&id));
        }
        Watcher::publish_state(&items, &hub).await;
    });
}
