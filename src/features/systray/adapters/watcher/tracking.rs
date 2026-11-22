use super::Watcher;
use crate::features::systray::adapters::fetcher::fetch_systray_item;
use crate::features::systray::adapters::signal_loop::run_signal_loop;
use crate::features::systray::adapters::sni_proxy::StatusNotifierItemProxy;
use crate::features::systray::adapters::tray_items::TrayItems;
use crate::features::systray::domain::SystrayAddress;
use tracing::Instrument;
use zbus::Connection;

impl Watcher {
    /// Tracks a new status notifier item, fetching its initial state and spawning its event loop.
    ///
    /// # Errors
    ///
    /// Returns a [`zbus::Error`] if creating the proxy or establishing D-Bus tracking fails.
    pub(crate) async fn track_item(
        conn: Connection,
        tray: TrayItems,
        address: SystrayAddress,
    ) -> zbus::Result<()> {
        let span = tracing::info_span!(
            "track_item",
            dest = ?address.destination().as_str(),
            path_str = ?address.path().as_str()
        );
        Self::fetch_and_follow(conn, tray, address)
            .instrument(span)
            .await
    }

    async fn fetch_and_follow(
        conn: Connection,
        tray: TrayItems,
        address: SystrayAddress,
    ) -> zbus::Result<()> {
        let id = address.id();

        let proxy = StatusNotifierItemProxy::builder(&conn)
            .destination(address.destination().as_str().to_owned())
            .path(address.path().as_str().to_owned())
            .build()
            .await?;

        let item = fetch_systray_item(&conn, address).await;

        {
            let mut lock = tray.items().write().await;
            lock.insert(id.clone(), item);
        }
        tray.publish().await;

        tracing::debug!("Setting up SNI signal streams for {}", id.as_str());
        run_signal_loop(proxy, tray, id.as_str().to_owned()).await;

        Ok(())
    }

    pub(crate) async fn remove_by_destination(tray: &TrayItems, destination: &str) -> bool {
        let keys_to_remove: Vec<_> = {
            let lock = tray.items().read().await;
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
            let mut lock = tray.items().write().await;
            for key in keys_to_remove {
                tracing::debug!(
                    "Removing SNI systray item {} because D-Bus name {destination} disconnected",
                    key.as_str()
                );
                lock.remove(&key);
                removed = true;
            }
            drop(lock);
            tray.publish().await;
        }
        removed
    }
}
