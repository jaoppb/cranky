mod bus;

use super::action_invoker::invoke_action;
use super::tray_items::TrayItems;
use super::watcher::Watcher;
use crate::features::systray::domain::{SystrayActionCall, SystrayId, SystrayItem};
use crate::features::systray::ports::{SniPort, SniPortError};
use crate::shared::events::signals::SignalHub;
use async_trait::async_trait;
use bus::{follow_name_owners, item_proxy, session};
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

fn log_no_dbus_proxy() {
    error!("Failed to create DBusProxy for NameOwnerChanged monitoring");
}

fn log_no_name_owner_changed() {
    error!("Failed to subscribe to NameOwnerChanged");
}

fn log_watcher_claimed() {
    info!("Successfully claimed org.kde.StatusNotifierWatcher");
}

fn log_host_only() {
    info!("Could not claim org.kde.StatusNotifierWatcher. Will attempt to run as host only.");
}

#[async_trait]
impl SniPort for SniAdapter {
    async fn start(&mut self) -> Result<(), SniPortError> {
        let conn = session().await?;
        tokio::spawn(follow_name_owners(self.tray.clone(), conn.clone()));
        self.claim_watcher(&conn).await?;
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
    /// Becomes the `StatusNotifierWatcher` when the name is free; otherwise the tray runs as a
    /// host only.
    async fn claim_watcher(&self, conn: &Connection) -> Result<(), SniPortError> {
        // Attempt to request the Watcher name
        if conn
            .request_name("org.kde.StatusNotifierWatcher")
            .await
            .is_ok()
        {
            self.serve_watcher(conn).await
        } else {
            log_host_only();
            Ok(())
        }
    }

    /// Serves the `StatusNotifierWatcher` interface, once its name is ours.
    async fn serve_watcher(&self, conn: &Connection) -> Result<(), SniPortError> {
        log_watcher_claimed();
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
        Ok(())
    }

    /// The connection, when started, and the item `id`, when tracked.
    async fn lookup(&self, id: &SystrayId) -> (Option<Connection>, Option<SystrayItem>) {
        let lock = self.conn.lock().await;
        let items_lock = self.tray.items().read().await;
        (lock.clone(), items_lock.get(id).cloned())
    }

    async fn route_action(
        &self,
        id: &SystrayId,
        call: &SystrayActionCall,
    ) -> Result<(), SniPortError> {
        let (conn_opt, item_opt) = self.lookup(id).await;
        if let (Some(conn), Some(item)) = (conn_opt.as_ref(), item_opt.as_ref()) {
            log_routing(id, item, call);
            let proxy = item_proxy(conn, item, id).await?;
            invoke_action(&proxy, item, call).await;
        } else {
            if conn_opt.is_none() {
                log_no_connection(id, call);
            }
            if item_opt.is_none() {
                log_unknown_item(id, call);
            }
        }
        Ok(())
    }
}

fn proxy_failed(id: &SystrayId, error: &zbus::Error) -> SniPortError {
    error!(
        "trigger_action: Failed to create D-Bus proxy for {}: {error}",
        id.as_str()
    );
    SniPortError::ActionFailed {
        id: id.as_str().to_string(),
        error: error.to_string(),
    }
}

fn log_routing(id: &SystrayId, item: &SystrayItem, call: &SystrayActionCall) {
    let pos = call.position();
    debug!(
        "trigger_action: Systray item found [id={}, dest={}, path={}, item_is_menu={}], routing action '{}' at pos {pos:?}",
        id.as_str(),
        item.destination().as_str(),
        item.path().as_str(),
        bool::from(item.item_is_menu()),
        call.name().as_str()
    );
}

fn log_no_connection(id: &SystrayId, call: &SystrayActionCall) {
    error!(
        "trigger_action: No D-Bus connection available when trying to trigger action '{}' on {}",
        call.name().as_str(),
        id.as_str()
    );
}

fn log_unknown_item(id: &SystrayId, call: &SystrayActionCall) {
    error!(
        "trigger_action: Systray item ID '{}' not found in registry when trying to trigger action '{}'",
        id.as_str(),
        call.name().as_str()
    );
}

#[cfg(test)]
mod tests {
    use super::SniAdapter;
    use crate::features::module_runtime::test_support::capture_logs;
    use crate::features::systray::adapters::tests::fixtures::item;
    use crate::features::systray::domain::{SystrayActionCall, SystrayActionName, SystrayId};
    use crate::features::systray::ports::SniPort;
    use crate::shared::config::domain::Config;
    use crate::shared::events::signals::SignalHub;
    use std::sync::Arc;

    #[test]
    fn test_trigger_action_logs_a_missing_connection_and_an_unknown_item() {
        let adapter = SniAdapter::new(Arc::new(SignalHub::new(Config::default())));
        let call = SystrayActionCall::new(SystrayActionName::Activate, None);
        let logs = capture_logs(|| {
            let id = SystrayId::new("ghost");
            let result = crate::test_utils::block_on(adapter.trigger_action(&id, &call));
            assert!(result.is_ok());
        });
        let span = "trigger_action{id=SystrayId(\"ghost\") action=Activate pos=None}";
        let target = "cranky::features::systray::adapters::sni_adapter";
        assert_eq!(
            logs,
            format!(
                "ERROR {span}: {target}: trigger_action: No D-Bus connection available when \
                 trying to trigger action 'Activate' on ghost\n\
                 ERROR {span}: {target}: trigger_action: Systray item ID 'ghost' not found in \
                 registry when trying to trigger action 'Activate'\n"
            )
        );
    }

    #[test]
    fn test_trigger_action_on_a_tracked_item_without_a_connection_logs_only_the_connection() {
        let adapter = SniAdapter::new(Arc::new(SignalHub::new(Config::default())));
        let id = SystrayId::new("app");
        let tracked = item("app", ":1.42");
        crate::test_utils::block_on(adapter.tray.insert_and_publish(id.clone(), tracked));
        let call = SystrayActionCall::new(SystrayActionName::Activate, None);
        let logs = capture_logs(|| {
            let result = crate::test_utils::block_on(adapter.trigger_action(&id, &call));
            assert!(result.is_ok());
        });
        let span = "trigger_action{id=SystrayId(\"app\") action=Activate pos=None}";
        let target = "cranky::features::systray::adapters::sni_adapter";
        assert_eq!(
            logs,
            format!(
                "ERROR {span}: {target}: trigger_action: No D-Bus connection available when \
                 trying to trigger action 'Activate' on app\n"
            )
        );
    }
}
