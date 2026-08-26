use std::sync::Arc;
use tokio::sync::watch;
use tokio_stream::StreamExt;
use tracing::debug;

use crate::shared::dbus::domain::{DBusState, DBusSubscription};
use crate::shared::dbus::ports::{DbusConnectionError, DbusConnectionPort};
use crate::shared::events::signals::SignalHub;

pub struct DbusSubscriptionManager {
    conn: Arc<dyn DbusConnectionPort>,
    dbus_tx: watch::Sender<DBusState>,
}

impl DbusSubscriptionManager {
    #[must_use]
    pub fn new(conn: Arc<dyn DbusConnectionPort>, hub: &SignalHub) -> Self {
        Self {
            conn,
            dbus_tx: hub.dbus_tx(),
        }
    }

    /// Subscribes to `DBus` signals based on the provided subscription specification.
    ///
    /// # Errors
    ///
    /// Returns `DbusConnectionError` if subscribing on the underlying `DBus` connection fails.
    pub async fn subscribe(&mut self, sub: DBusSubscription) -> Result<(), DbusConnectionError> {
        let is_properties_changed = sub
            .member()
            .is_some_and(|m| m.as_str() == "PropertiesChanged");

        let tx = self.dbus_tx.clone();

        if is_properties_changed && let (Some(dest), Some(path)) = (sub.destination(), sub.path()) {
            let mut stream = self
                .conn
                .subscribe_properties_changed(sub.bus(), dest, path)
                .await?;
            debug!("Subscribed to DBus PropertiesChanged on {path:?}");
            let path_str = path.as_str().to_string();
            tokio::spawn(async move {
                while let Some((iface, changed_props)) = stream.next().await {
                    let iface_str = iface.as_str();
                    tracing::debug!(
                        "Received DBus PropertiesChanged on {path_str:?} for interface {iface_str:?}"
                    );
                    let mut properties = tx.borrow().properties().clone();
                    for (k, v) in &changed_props {
                        let k_str = k.as_str();
                        let prop_key = format!("{iface_str}.{k_str}");
                        properties.insert(prop_key, v.clone());
                    }
                    let _ = tx.send(DBusState::new(properties));
                }
            });
            return Ok(());
        }

        // Otherwise, generic signal
        let mut stream = self.conn.subscribe_signal(sub).await?;
        debug!("Subscribed to generic DBus signal");
        tokio::spawn(async move {
            while let Some((path, member, value)) = stream.next().await {
                let path_str = path.as_str();
                let member_str = member.as_str();
                tracing::debug!(
                    "Received generic DBus signal on {path_str:?} for member {member_str:?}"
                );
                let mut properties = tx.borrow().properties().clone();
                let prop_key = format!("{path_str}.{member_str}");
                properties.insert(prop_key, value);
                let _ = tx.send(DBusState::new(properties));
            }
        });

        Ok(())
    }
}
