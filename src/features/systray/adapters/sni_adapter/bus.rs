//! The D-Bus plumbing of the host: its session connection, the bus's name-owner changes, and a
//! proxy on each item. Failures are logged by the parent module, under its target.
use super::{log_no_dbus_proxy, log_no_name_owner_changed, proxy_failed};
use crate::features::systray::adapters::tray_items::TrayItems;
use crate::features::systray::adapters::watcher::Watcher;
use crate::features::systray::domain::{SystrayId, SystrayItem};
use crate::features::systray::ports::SniPortError;
use zbus::Connection;
use zbus::fdo::NameOwnerChangedStream;

/// The session bus connection the tray talks over.
pub(super) async fn session() -> Result<Connection, SniPortError> {
    Connection::session()
        .await
        .map_err(|error| SniPortError::StartFailed(error.to_string()))
}

/// The bus's `NameOwnerChanged` signals, or `None` after logging why they are unavailable.
async fn name_owner_changes(conn: &Connection) -> Option<NameOwnerChangedStream> {
    let Ok(proxy) = zbus::fdo::DBusProxy::new(conn).await else {
        log_no_dbus_proxy();
        return None;
    };
    let Ok(stream) = proxy.receive_name_owner_changed().await else {
        log_no_name_owner_changed();
        return None;
    };
    Some(stream)
}

/// Drops every tracked item whose bus name lost its owner, for as long as the bus reports owner
/// changes.
pub(super) async fn follow_name_owners(tray: TrayItems, conn: Connection) {
    use tokio_stream::StreamExt;
    let Some(mut stream) = name_owner_changes(&conn).await else {
        return;
    };
    while let Some(sig) = stream.next().await {
        if let Ok(args) = sig.args()
            && args
                .new_owner()
                .as_ref()
                .is_none_or(|owner| owner.as_str().is_empty())
        {
            Watcher::remove_by_destination(&tray, args.name()).await;
        }
    }
}

/// A proxy on the item's `org.kde.StatusNotifierItem` interface; a failure is logged and becomes
/// [`SniPortError::ActionFailed`].
pub(super) async fn item_proxy<'conn>(
    conn: &'conn Connection,
    item: &SystrayItem,
    id: &SystrayId,
) -> Result<zbus::Proxy<'conn>, SniPortError> {
    zbus::Proxy::new(
        conn,
        item.destination().as_str().to_string(),
        item.path().as_str().to_string(),
        "org.kde.StatusNotifierItem",
    )
    .await
    .map_err(|error: zbus::Error| proxy_failed(id, &error))
}
