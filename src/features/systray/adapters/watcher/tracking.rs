use super::Watcher;
use crate::features::systray::adapters::fetcher::fetch_systray_item;
use crate::features::systray::adapters::signal_loop::run_signal_loop;
use crate::features::systray::adapters::sni_proxy::StatusNotifierItemProxy;
use crate::features::systray::domain::{SystrayId, SystrayItem, SystrayState};
use crate::shared::events::signals::SignalHub;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::Instrument;
use zbus::Connection;

impl Watcher {
    /// Tracks a new status notifier item, fetching its initial state and spawning its event loop.
    ///
    /// # Errors
    ///
    /// Returns a [`zbus::Error`] if creating the proxy or establishing D-Bus tracking fails.
    pub async fn track_item(
        conn: Connection,
        items: Arc<RwLock<BTreeMap<SystrayId, SystrayItem>>>,
        hub: Arc<SignalHub>,
        dest: String,
        path_str: String,
    ) -> zbus::Result<()> {
        let span = tracing::info_span!("track_item", ?dest, ?path_str);
        Self::fetch_and_follow(conn, items, hub, dest, path_str)
            .instrument(span)
            .await
    }

    async fn fetch_and_follow(
        conn: Connection,
        items: Arc<RwLock<BTreeMap<SystrayId, SystrayItem>>>,
        hub: Arc<SignalHub>,
        dest: String,
        path_str: String,
    ) -> zbus::Result<()> {
        let id = format!("{dest}{path_str}");

        let proxy = StatusNotifierItemProxy::builder(&conn)
            .destination(dest.clone())
            .path(path_str.clone())
            .build()
            .await?;

        let item = fetch_systray_item(&conn, id.clone(), dest.clone(), path_str.clone()).await;

        {
            let mut lock = items.write().await;
            lock.insert(SystrayId::new(&id), item);
        }
        Self::publish_state(&items, &hub).await;

        tracing::debug!("Setting up SNI signal streams for {id}");
        run_signal_loop(proxy, items, hub, id).await;

        Ok(())
    }

    pub async fn remove_by_destination(
        items: &Arc<RwLock<BTreeMap<SystrayId, SystrayItem>>>,
        hub: &Arc<SignalHub>,
        destination: &str,
    ) -> bool {
        let keys_to_remove: Vec<_> = {
            let lock = items.read().await;
            lock.iter()
                .filter_map(|(id, item)| {
                    if item.destination().as_str() == destination {
                        Some(id.clone())
                    } else {
                        None
                    }
                })
                .collect()
        };

        let mut removed = false;
        if !keys_to_remove.is_empty() {
            let mut lock = items.write().await;
            for key in keys_to_remove {
                tracing::debug!(
                    "Removing SNI systray item {} because D-Bus name {destination} disconnected",
                    key.as_str()
                );
                lock.remove(&key);
                removed = true;
            }
            drop(lock);
            Self::publish_state(items, hub).await;
        }
        removed
    }

    pub async fn publish_state(
        items: &Arc<RwLock<BTreeMap<SystrayId, SystrayItem>>>,
        hub: &Arc<SignalHub>,
    ) {
        let state = {
            let lock = items.read().await;
            SystrayState::new(lock.clone())
        };
        let _ = hub.systray_tx().send(state);
    }
}
