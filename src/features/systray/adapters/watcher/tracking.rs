use super::Watcher;
use crate::features::systray::adapters::fetcher::fetch_systray_item;
use crate::features::systray::adapters::signal_loop::run_signal_loop;
use crate::features::systray::adapters::sni_proxy::StatusNotifierItemProxy;
use crate::features::systray::adapters::tray_items::TrayItems;
use crate::features::systray::domain::{SystrayAddress, SystrayId};
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
        let proxy = item_proxy(&conn, &address).await?;
        let item = fetch_systray_item(&conn, address).await;
        tray.insert_and_publish(id.clone(), item).await;
        log_following(&id);
        run_signal_loop(proxy, tray, id.as_str().to_owned()).await;
        Ok(())
    }

    pub(crate) async fn remove_by_destination(tray: &TrayItems, destination: &str) -> bool {
        let keys_to_remove = ids_at(tray, destination).await;
        if keys_to_remove.is_empty() {
            return false;
        }
        {
            let mut lock = tray.items().write().await;
            for key in keys_to_remove {
                log_removing(&key, destination);
                lock.remove(&key);
            }
        }
        tray.publish().await;
        true
    }
}

/// A proxy on the item's `org.kde.StatusNotifierItem` interface, for its signals.
async fn item_proxy(
    conn: &Connection,
    address: &SystrayAddress,
) -> zbus::Result<StatusNotifierItemProxy<'static>> {
    StatusNotifierItemProxy::builder(conn)
        .destination(address.destination().as_str().to_owned())
        .path(address.path().as_str().to_owned())
        .build()
        .await
}

/// The ids of every tracked item the bus name `destination` serves.
async fn ids_at(tray: &TrayItems, destination: &str) -> Vec<SystrayId> {
    let lock = tray.items().read().await;
    lock.iter()
        .filter(|(_, item)| item.destination().as_str() == destination)
        .map(|(id, _)| id.clone())
        .collect()
}

fn log_following(id: &SystrayId) {
    tracing::debug!("Setting up SNI signal streams for {}", id.as_str());
}

fn log_removing(key: &SystrayId, destination: &str) {
    tracing::debug!(
        "Removing SNI systray item {} because D-Bus name {destination} disconnected",
        key.as_str()
    );
}
