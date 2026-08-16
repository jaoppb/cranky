use std::sync::Arc;
use tokio::sync::watch;
use tokio_stream::StreamExt;
use tracing::info;

use crate::shared::dbus::domain::{DBusState, DBusSubscription};
use crate::shared::dbus::ports::{DbusConnectionError, DbusConnectionPort};
use crate::shared::events::signals::SignalHub;

pub struct DbusSubscriptionManager {
    conn: Arc<dyn DbusConnectionPort>,
    dbus_tx: watch::Sender<DBusState>,
}

impl DbusSubscriptionManager {
    pub fn new(conn: Arc<dyn DbusConnectionPort>, hub: &SignalHub) -> Self {
        Self {
            conn,
            dbus_tx: hub.dbus_tx(),
        }
    }

    pub async fn subscribe(&mut self, sub: DBusSubscription) -> Result<(), DbusConnectionError> {
        let is_properties_changed = sub.member().map(|m| m.as_str() == "PropertiesChanged").unwrap_or(false);

        let tx = self.dbus_tx.clone();
        
        if is_properties_changed
            && let (Some(dest), Some(path)) = (sub.destination(), sub.path()) {
                let mut stream = self.conn.subscribe_properties_changed(sub.bus(), dest, path).await?;
                info!("Subscribed to DBus PropertiesChanged on {:?}", path);
                let path_str = path.as_str().to_string();
                tokio::spawn(async move {
                    while let Some((iface, changed_props)) = stream.next().await {
                        tracing::debug!("Received DBus PropertiesChanged on {:?} for interface {:?}", path_str, iface.as_str());
                        let mut properties = tx.borrow().properties().clone();
                        for (k, v) in changed_props.iter() {
                            let prop_key = format!("{}.{}", iface.as_str(), k.as_str());
                            properties.insert(prop_key, v.clone());
                        }
                        let _ = tx.send(DBusState::new(properties));
                    }
                });
                return Ok(());
            }
        
        // Otherwise, generic signal
        let mut stream = self.conn.subscribe_signal(sub).await?;
        info!("Subscribed to generic DBus signal");
        tokio::spawn(async move {
            while let Some((path, member, value)) = stream.next().await {
                tracing::debug!("Received generic DBus signal on {:?} for member {:?}", path.as_str(), member.as_str());
                let mut properties = tx.borrow().properties().clone();
                let prop_key = format!("{}.{}", path.as_str(), member.as_str());
                properties.insert(prop_key, value);
                let _ = tx.send(DBusState::new(properties));
            }
        });

        Ok(())
    }
}
