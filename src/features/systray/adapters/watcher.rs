use super::fetcher::fetch_systray_item;
use super::signal_loop::run_signal_loop;
use super::sni_proxy::StatusNotifierItemProxy;
use crate::features::systray::domain::{SystrayId, SystrayItem, SystrayState};
use crate::shared::events::signals::SignalHub;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error};
use zbus::{Connection, interface};

pub struct Watcher {
    pub items: Arc<RwLock<BTreeMap<SystrayId, SystrayItem>>>,
    pub hub: Arc<SignalHub>,
    pub conn: Connection,
    pub runtime: tokio::runtime::Handle,
}

#[interface(name = "org.kde.StatusNotifierWatcher")]
impl Watcher {
    #[allow(clippy::unused_async)]
    async fn register_status_notifier_item(
        &self,
        service: String,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) {
        debug!("Registered SNI item: {service}");

        let mut full_path = service.clone();
        if !full_path.starts_with('/') {
            full_path = "/StatusNotifierItem".to_string();
        }

        let dbus_dest = if service.starts_with('/') {
            header
                .sender()
                .map_or_else(|| service.clone(), |s| s.as_str().to_string())
        } else {
            service
        };

        let conn = self.conn.clone();
        let items = self.items.clone();
        let hub = self.hub.clone();

        self.runtime.spawn(async move {
            if let Err(e) = Self::track_item(conn, items, hub, dbus_dest, full_path).await {
                error!("Failed to track SNI item: {e}");
            }
        });
    }

    #[allow(clippy::unused_async)]
    async fn register_status_notifier_host(&self, service: String) {
        debug!("Registered SNI host: {service}");
    }

    #[zbus(property)]
    async fn registered_status_notifier_items(&self) -> Vec<String> {
        let items = self.items.read().await;
        items.keys().map(|id| id.as_str().to_string()).collect()
    }

    #[allow(clippy::unused_self)]
    #[zbus(property)]
    const fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    #[allow(clippy::unused_self)]
    #[zbus(property)]
    const fn protocol_version(&self) -> i32 {
        0
    }
}

impl Watcher {
    /// Tracks a new status notifier item, fetching its initial state and spawning its event loop.
    ///
    /// # Errors
    ///
    /// Returns a [`zbus::Error`] if creating the proxy or establishing D-Bus tracking fails.
    #[tracing::instrument(skip(conn, items, hub))]
    pub async fn track_item(
        conn: Connection,
        items: Arc<RwLock<BTreeMap<SystrayId, SystrayItem>>>,
        hub: Arc<SignalHub>,
        dest: String,
        path_str: String,
    ) -> zbus::Result<()> {
        let id = format!("{dest}{path_str}");

        let proxy = StatusNotifierItemProxy::builder(&conn)
            .destination(dest.clone())?
            .path(path_str.clone())?
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
