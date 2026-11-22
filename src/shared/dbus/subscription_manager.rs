//! Turns script `DBus` subscriptions into streams that update the hub's `DBus` state.

use std::sync::Arc;
use tokio::sync::watch;
use tokio_stream::StreamExt;
use tracing::debug;

use crate::shared::dbus::domain::{BusObject, DBusState, DBusSubscription};
use crate::shared::dbus::ports::{DbusConnectionError, DbusConnectionPort};
use crate::shared::events::signals::SignalHub;

pub(crate) struct DbusSubscriptionManager {
    conn: Arc<dyn DbusConnectionPort>,
    dbus_tx: watch::Sender<DBusState>,
}

impl DbusSubscriptionManager {
    #[must_use]
    pub(crate) fn new(conn: Arc<dyn DbusConnectionPort>, hub: &SignalHub) -> Self {
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
    pub(crate) async fn subscribe(&self, sub: DBusSubscription) -> Result<(), DbusConnectionError> {
        let is_properties_changed = sub
            .member()
            .is_some_and(|member| member.as_str() == "PropertiesChanged");

        let tx = self.dbus_tx.clone();

        if is_properties_changed && let (Some(dest), Some(path)) = (sub.destination(), sub.path()) {
            let mut stream = self
                .conn
                .subscribe_properties_changed(&BusObject::new(
                    sub.bus(),
                    dest.clone(),
                    path.clone(),
                ))
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
                    for (key, value) in &changed_props {
                        let k_str = key.as_str();
                        let prop_key = format!("{iface_str}.{k_str}");
                        properties.insert(prop_key, value.clone());
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use tokio::sync::watch;

    use super::DbusSubscriptionManager;
    use crate::shared::config::domain::Config;
    use crate::shared::dbus::domain::{
        BusObject, BusType, DBusState, DBusSubscription, DBusValue, Destination, Interface, Member,
        NameChangedStream, Path, PropertiesMap, PropertyChangedStream, PropertyName,
        QualifiedMember, SignalStream,
    };
    use crate::shared::dbus::ports::{DbusConnectionError, DbusConnectionPort, DisconnectedBus};
    use crate::shared::events::signals::SignalHub;

    /// A bus that replays canned signals and records what it was asked to subscribe to.
    #[derive(Default)]
    struct ReplayBus {
        watched: Mutex<Vec<BusObject>>,
        signals: Mutex<Vec<DBusSubscription>>,
    }

    fn changed(entries: &[(&str, DBusValue)]) -> (Interface, PropertiesMap) {
        let map = entries
            .iter()
            .map(|(name, value)| (PropertyName::new(*name), value.clone()))
            .collect();
        (Interface::new("org.Player"), PropertiesMap::new(map))
    }

    #[async_trait]
    impl DbusConnectionPort for ReplayBus {
        async fn call_method(
            &self,
            object: &BusObject,
            _method: &QualifiedMember,
        ) -> Result<DBusValue, DbusConnectionError> {
            Err(DbusConnectionError::NotInitialized(object.bus()))
        }

        async fn get_all_properties(
            &self,
            object: &BusObject,
            _interface: &Interface,
        ) -> Result<PropertiesMap, DbusConnectionError> {
            Err(DbusConnectionError::NotInitialized(object.bus()))
        }

        async fn subscribe_properties_changed(
            &self,
            object: &BusObject,
        ) -> Result<PropertyChangedStream, DbusConnectionError> {
            self.watched.lock().unwrap().push(object.clone());
            Ok(Box::pin(futures::stream::iter([
                changed(&[("Volume", DBusValue::Int(3))]),
                changed(&[
                    ("Volume", DBusValue::Int(4)),
                    ("Title", DBusValue::String("song".to_string())),
                ]),
            ])))
        }

        async fn list_names(&self, bus: BusType) -> Result<Vec<Destination>, DbusConnectionError> {
            Err(DbusConnectionError::NotInitialized(bus))
        }

        async fn subscribe_name_changes(
            &self,
            bus: BusType,
        ) -> Result<NameChangedStream, DbusConnectionError> {
            Err(DbusConnectionError::NotInitialized(bus))
        }

        async fn subscribe_signal(
            &self,
            sub: DBusSubscription,
        ) -> Result<SignalStream, DbusConnectionError> {
            self.signals.lock().unwrap().push(sub);
            Ok(Box::pin(futures::stream::iter([
                (
                    Path::new("/org/probe"),
                    Member::new("Pinged"),
                    DBusValue::Int(1),
                ),
                (
                    Path::new("/org/probe"),
                    Member::new("Pinged"),
                    DBusValue::Int(2),
                ),
            ])))
        }
    }

    fn subscription(member: &str, path: Option<&str>) -> DBusSubscription {
        DBusSubscription::new(BusType::Session)
            .with_destination(Some(Destination::new("org.probe")))
            .with_path(path.map(Path::new))
            .with_member(Some(Member::new(member)))
    }

    /// A hub whose `DBus` state already holds `keep`, which every update must carry over.
    fn hub_with_a_kept_property() -> SignalHub {
        let hub = SignalHub::new(Config::default());
        let kept = HashMap::from([("keep".to_string(), DBusValue::Bool(true))]);
        hub.dbus_tx().send(DBusState::new(kept)).unwrap();
        hub
    }

    /// Waits up to five seconds for the hub's `DBus` properties to equal `expected`.
    async fn reaches(rx: &mut watch::Receiver<DBusState>, expected: &[(&str, DBusValue)]) -> bool {
        let expected: HashMap<String, DBusValue> = expected
            .iter()
            .map(|(key, value)| ((*key).to_string(), value.clone()))
            .collect();
        let wait = async {
            while rx.borrow_and_update().properties() != &expected {
                rx.changed().await.unwrap();
            }
        };
        tokio::time::timeout(std::time::Duration::from_secs(5), wait)
            .await
            .is_ok()
    }

    #[test]
    fn properties_changed_merges_each_change_under_its_interface() {
        crate::test_utils::block_on(async {
            let hub = hub_with_a_kept_property();
            let bus = Arc::new(ReplayBus::default());
            let manager = DbusSubscriptionManager::new(bus.clone(), &hub);

            let sub = subscription("PropertiesChanged", Some("/org/probe"));
            manager.subscribe(sub).await.unwrap();

            let mut rx = hub.dbus_rx();
            let expected = [
                ("keep", DBusValue::Bool(true)),
                ("org.Player.Volume", DBusValue::Int(4)),
                ("org.Player.Title", DBusValue::String("song".to_string())),
            ];
            assert!(reaches(&mut rx, &expected).await);
            let object = BusObject::new(
                BusType::Session,
                Destination::new("org.probe"),
                Path::new("/org/probe"),
            );
            assert_eq!(*bus.watched.lock().unwrap(), [object]);
            assert!(bus.signals.lock().unwrap().is_empty());
        });
    }

    #[test]
    fn a_generic_signal_is_stored_under_its_path_and_member() {
        crate::test_utils::block_on(async {
            let hub = hub_with_a_kept_property();
            let bus = Arc::new(ReplayBus::default());
            let manager = DbusSubscriptionManager::new(bus.clone(), &hub);

            let sub = subscription("Pinged", Some("/org/probe"));
            manager.subscribe(sub.clone()).await.unwrap();

            let mut rx = hub.dbus_rx();
            let expected = [
                ("keep", DBusValue::Bool(true)),
                ("/org/probe.Pinged", DBusValue::Int(2)),
            ];
            assert!(reaches(&mut rx, &expected).await);
            assert_eq!(*bus.signals.lock().unwrap(), [sub]);
            assert!(bus.watched.lock().unwrap().is_empty());
        });
    }

    #[test]
    fn properties_changed_without_a_path_is_subscribed_as_a_generic_signal() {
        crate::test_utils::block_on(async {
            let hub = hub_with_a_kept_property();
            let bus = Arc::new(ReplayBus::default());
            let manager = DbusSubscriptionManager::new(bus.clone(), &hub);

            let sub = subscription("PropertiesChanged", None);
            manager.subscribe(sub.clone()).await.unwrap();

            assert_eq!(*bus.signals.lock().unwrap(), [sub]);
            assert!(bus.watched.lock().unwrap().is_empty());
        });
    }

    #[test]
    fn a_subscription_the_bus_refuses_returns_its_error() {
        crate::test_utils::block_on(async {
            let hub = SignalHub::new(Config::default());
            let manager = DbusSubscriptionManager::new(Arc::new(DisconnectedBus), &hub);

            let properties = manager
                .subscribe(subscription("PropertiesChanged", Some("/org/probe")))
                .await;
            let generic = manager.subscribe(subscription("Pinged", None)).await;

            assert!(matches!(
                properties,
                Err(DbusConnectionError::NotInitialized(BusType::Session))
            ));
            assert!(matches!(
                generic,
                Err(DbusConnectionError::SubscriptionFailed(_))
            ));
        });
    }
}
