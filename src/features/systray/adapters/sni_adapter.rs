use super::action_invoker::invoke_action;
use super::watcher::Watcher;
use crate::features::systray::domain::{SystrayActionName, SystrayId, SystrayItem};
use crate::features::systray::ports::{SniPort, SniPortError};
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::geometry::Position;
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use zbus::Connection;

#[derive(Clone)]
pub struct SniAdapter {
    hub: Arc<SignalHub>,
    conn: Arc<tokio::sync::Mutex<Option<Connection>>>,
    items: Arc<RwLock<BTreeMap<SystrayId, SystrayItem>>>,
}

impl SniAdapter {
    #[must_use]
    pub fn new(hub: Arc<SignalHub>) -> Self {
        Self {
            hub,
            conn: Arc::new(tokio::sync::Mutex::new(None)),
            items: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }
}

#[async_trait]
impl SniPort for SniAdapter {
    async fn start(&mut self) -> Result<(), SniPortError> {
        let conn = Connection::session()
            .await
            .map_err(|e| SniPortError::StartFailed(e.to_string()))?;

        let items_clone = self.items.clone();
        let hub_clone = self.hub.clone();
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
                        .is_none_or(|n| n.as_str().is_empty());
                    if is_unowned {
                        Watcher::remove_by_destination(&items_clone, &hub_clone, args.name()).await;
                    }
                }
            }
        });

        // Attempt to request the Watcher name
        match conn.request_name("org.kde.StatusNotifierWatcher").await {
            Ok(()) => {
                info!("Successfully claimed org.kde.StatusNotifierWatcher");
                let watcher = Watcher {
                    items: self.items.clone(),
                    hub: self.hub.clone(),
                    conn: conn.clone(),
                    runtime: tokio::runtime::Handle::current(),
                };
                let _res: bool = conn
                    .object_server()
                    .at::<&str, Watcher>("/StatusNotifierWatcher", watcher)
                    .await
                    .map_err(|e: zbus::Error| SniPortError::StartFailed(e.to_string()))?;
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

    #[tracing::instrument(skip(self))]
    async fn trigger_action(
        &self,
        id: &SystrayId,
        action: &SystrayActionName,
        pos: Option<Position>,
    ) -> Result<(), SniPortError> {
        let (conn_opt, item_opt) = {
            let lock = self.conn.lock().await;
            let items_lock = self.items.read().await;
            (lock.clone(), items_lock.get(id).cloned())
        };

        if let (Some(conn), Some(item)) = (conn_opt.as_ref(), item_opt.as_ref()) {
            debug!(
                "trigger_action: Systray item found [id={}, dest={}, path={}, item_is_menu={}], routing action '{}' at pos {pos:?}",
                id.as_str(),
                item.destination().as_str(),
                item.path().as_str(),
                item.item_is_menu().value(),
                action.as_str()
            );
            let proxy = zbus::Proxy::new(
                conn,
                item.destination().as_str().to_string(),
                item.path().as_str().to_string(),
                "org.kde.StatusNotifierItem",
            )
            .await
            .map_err(|e: zbus::Error| {
                error!(
                    "trigger_action: Failed to create D-Bus proxy for {}: {e}",
                    id.as_str()
                );
                SniPortError::ActionFailed {
                    id: id.as_str().to_string(),
                    error: e.to_string(),
                }
            })?;

            invoke_action(&proxy, item, action, pos).await;
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
