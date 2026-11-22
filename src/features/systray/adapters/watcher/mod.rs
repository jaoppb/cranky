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

#[interface(name = "org.kde.StatusNotifierWatcher")]
impl Watcher {
    fn register_status_notifier_item(
        &self,
        service: &str,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) {
        debug!("Registered SNI item: {service}");

        let full_path = if service.starts_with('/') {
            service.to_string()
        } else {
            "/StatusNotifierItem".to_string()
        };

        let dbus_dest = if service.starts_with('/') {
            Sender::from(header)
                .0
                .unwrap_or_else(|| service.to_string())
        } else {
            service.to_string()
        };

        let conn = self.conn.clone();
        let tray = self.tray.clone();
        let address = SystrayAddress::new(Destination::new(dbus_dest), ObjectPath::new(full_path));

        self.runtime.spawn(async move {
            if let Err(error) = Self::track_item(conn, tray, address).await {
                error!("Failed to track SNI item: {error}");
            }
        });
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
