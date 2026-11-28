mod tracking;

use super::tray_items::TrayItems;
use crate::features::systray::domain::{Destination, ObjectPath, SystrayAddress};
use tracing::{debug, error};
use zbus::{Connection, interface};

/// Which host the watcher reports as registered. Cranky is itself the host, so one is registered
/// from the start and never leaves while the watcher runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HostRegistration {
    /// Cranky's own host.
    Registered,
}

pub(super) struct Watcher {
    tray: TrayItems,
    conn: Connection,
    runtime: tokio::runtime::Handle,
    host: HostRegistration,
    protocol_version: i32,
}

/// The unique bus name a message was sent from, when it carries one.
struct Sender(Option<String>);

impl From<zbus::message::Header<'_>> for Sender {
    fn from(header: zbus::message::Header<'_>) -> Self {
        Self(header.sender().map(|name| name.as_str().to_string()))
    }
}

impl Watcher {
    #[must_use]
    pub(crate) const fn new(
        tray: TrayItems,
        conn: Connection,
        runtime: tokio::runtime::Handle,
    ) -> Self {
        Self {
            tray,
            conn,
            runtime,
            host: HostRegistration::Registered,
            protocol_version: 0,
        }
    }
}

/// Where a registering item lives. `service` is either an object path on the sender's
/// connection, as most items send, or a bus name whose item sits at `/StatusNotifierItem`.
fn item_address(service: &str, sender: Sender) -> SystrayAddress {
    if service.starts_with('/') {
        let destination = sender.0.unwrap_or_else(|| service.to_string());
        SystrayAddress::new(Destination::new(destination), ObjectPath::new(service))
    } else {
        SystrayAddress::new(
            Destination::new(service),
            ObjectPath::new("/StatusNotifierItem"),
        )
    }
}

/// Tracks the item at `address`, logging instead of returning a failure: nobody awaits the task.
async fn track_logged(conn: Connection, tray: TrayItems, address: SystrayAddress) {
    if let Err(error) = Watcher::track_item(conn, tray, address).await {
        log_track_failed(&error);
    }
}

/// Logs that tracking an item failed, out of line to keep its caller under `cognitive_complexity`.
fn log_track_failed(error: &zbus::Error) {
    error!("Failed to track SNI item: {error}");
}

#[interface(name = "org.kde.StatusNotifierWatcher")]
impl Watcher {
    fn register_status_notifier_item(
        &self,
        service: &str,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) {
        debug!("Registered SNI item: {service}");
        let address = item_address(service, Sender::from(header));
        self.runtime
            .spawn(track_logged(self.conn.clone(), self.tray.clone(), address));
    }

    fn register_status_notifier_host(&self, service: &str) {
        debug!(host = ?self.host, "Registered SNI host: {service}");
    }

    #[zbus(property)]
    async fn registered_status_notifier_items(&self) -> Vec<String> {
        let items = self.tray.items().read().await;
        items.keys().map(|id| id.as_str().to_string()).collect()
    }

    #[zbus(property)]
    const fn is_status_notifier_host_registered(&self) -> bool {
        matches!(self.host, HostRegistration::Registered)
    }

    #[zbus(property)]
    const fn protocol_version(&self) -> i32 {
        self.protocol_version
    }
}

#[cfg(test)]
mod tests {
    use super::{Sender, item_address};

    /// The destination and object path of the address `service` registers at, sent by `sender`.
    fn registered_at(service: &str, sender: Option<&str>) -> (String, String) {
        let address = item_address(service, Sender(sender.map(str::to_string)));
        (
            address.destination().as_str().to_string(),
            address.path().as_str().to_string(),
        )
    }

    #[test]
    fn test_item_address_of_an_object_path_is_on_the_sender() {
        let expected = (":1.7".to_string(), "/org/ayatana/item".to_string());
        assert_eq!(registered_at("/org/ayatana/item", Some(":1.7")), expected);
    }

    #[test]
    fn test_item_address_of_an_object_path_without_sender_uses_the_path_as_destination() {
        let expected = ("/item".to_string(), "/item".to_string());
        assert_eq!(registered_at("/item", None), expected);
    }

    #[test]
    fn test_item_address_of_a_bus_name_is_at_the_default_path() {
        let expected = (
            "org.kde.StatusNotifierItem-42-1".to_string(),
            "/StatusNotifierItem".to_string(),
        );
        let address = registered_at("org.kde.StatusNotifierItem-42-1", Some(":1.7"));
        assert_eq!(address, expected);
    }
}
