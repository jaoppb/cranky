use super::action_invoker::invoke_action;
use super::tray_items::TrayItems;
use super::watcher::Watcher;
use crate::features::systray::domain::{SystrayActionCall, SystrayId};
use crate::features::systray::ports::{SniPort, SniPortError};
use crate::shared::events::signals::SignalHub;
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{Instrument, debug, error, info};
use zbus::Connection;

/// The tray's SNI host on the session bus, which also becomes the `StatusNotifierWatcher` when no
/// other process holds that name.
///
/// Cloning shares the connection and the item registry, so every clone routes actions to the same
/// items.
#[derive(Debug, Clone)]
pub struct SniAdapter {
    tray: TrayItems,
    conn: Arc<tokio::sync::Mutex<Option<Connection>>>,
}

impl SniAdapter {
    /// Creates a host with no connection and no items; nothing touches D-Bus until
    /// [`SniPort::start`] runs.
    #[must_use]
    pub fn new(hub: Arc<SignalHub>) -> Self {
        Self {
            tray: TrayItems::new(Arc::new(RwLock::new(BTreeMap::new())), hub),
            conn: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }
}

#[async_trait]
impl SniPort for SniAdapter {
    async fn start(&mut self) -> Result<(), SniPortError> {
        let conn = Connection::session()
            .await
            .map_err(|error| SniPortError::StartFailed(error.to_string()))?;

        let tray_clone = self.tray.clone();
        let conn_clone = conn.clone();

        tokio::spawn(async move {
            use tokio_stream::StreamExt;
            let Ok(proxy) = zbus::fdo::DBusProxy::new(&conn_clone).await else {
                error!("Failed to create DBusProxy for NameOwnerChanged monitoring");
                return;
            };
            let Ok(mut stream) = proxy.receive_name_owner_changed().await else {
                error!("Failed to subscribe to NameOwnerChanged");
                return;
            };

            while let Some(sig) = stream.next().await {
                if let Ok(args) = sig.args() {
                    let is_unowned = args
                        .new_owner()
                        .as_ref()
                        .is_none_or(|owner| owner.as_str().is_empty());
                    if is_unowned {
                        Watcher::remove_by_destination(&tray_clone, args.name()).await;
                    }
                }
            }
        });

        // Attempt to request the Watcher name
        match conn.request_name("org.kde.StatusNotifierWatcher").await {
            Ok(()) => {
                info!("Successfully claimed org.kde.StatusNotifierWatcher");
                let watcher = Watcher::new(
                    self.tray.clone(),
                    conn.clone(),
                    tokio::runtime::Handle::current(),
                );
                let _rejected: Option<Watcher> = conn
                    .object_server()
                    .at::<&str, Watcher>("/StatusNotifierWatcher", watcher)
                    .await
                    .map_err(|error: zbus::Error| SniPortError::StartFailed(error.to_string()))?;
            }
            Err(_) => {
                info!(
                    "Could not claim org.kde.StatusNotifierWatcher. Will attempt to run as host only."
                );
            }
        }

        *self.conn.lock().await = Some(conn);
        Ok(())
    }

    async fn trigger_action(
        &self,
        id: &SystrayId,
        call: &SystrayActionCall,
    ) -> Result<(), SniPortError> {
        let action = call.name();
        let pos = call.position();
        self.route_action(id, call)
            .instrument(tracing::info_span!("trigger_action", ?id, ?action, ?pos))
            .await
    }
}

impl SniAdapter {
    async fn route_action(
        &self,
        id: &SystrayId,
        call: &SystrayActionCall,
    ) -> Result<(), SniPortError> {
        let action = call.name();
        let pos = call.position();
        let (conn_opt, item_opt) = {
            let lock = self.conn.lock().await;
            let items_lock = self.tray.items().read().await;
            (lock.clone(), items_lock.get(id).cloned())
        };

        if let (Some(conn), Some(item)) = (conn_opt.as_ref(), item_opt.as_ref()) {
            debug!(
                "trigger_action: Systray item found [id={}, dest={}, path={}, item_is_menu={}], routing action '{}' at pos {pos:?}",
                id.as_str(),
                item.destination().as_str(),
                item.path().as_str(),
                bool::from(item.item_is_menu()),
                action.as_str()
            );
            let proxy = zbus::Proxy::new(
                conn,
                item.destination().as_str().to_string(),
                item.path().as_str().to_string(),
                "org.kde.StatusNotifierItem",
            )
            .await
            .map_err(|error: zbus::Error| {
                error!(
                    "trigger_action: Failed to create D-Bus proxy for {}: {error}",
                    id.as_str()
                );
                SniPortError::ActionFailed {
                    id: id.as_str().to_string(),
                    error: error.to_string(),
                }
            })?;

            invoke_action(&proxy, item, call).await;
        } else {
            if conn_opt.is_none() {
                error!(
                    "trigger_action: No D-Bus connection available when trying to trigger action '{}' on {}",
                    action.as_str(),
                    id.as_str()
                );
            }
            if item_opt.is_none() {
                error!(
                    "trigger_action: Systray item ID '{}' not found in registry when trying to trigger action '{}'",
                    id.as_str(),
                    action.as_str()
                );
            }
        }
        Ok(())
    }
}
