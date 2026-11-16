mod tracking;

use crate::features::systray::domain::{SystrayId, SystrayItem};
use crate::shared::events::signals::SignalHub;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, error};
use zbus::{Connection, interface};

/// How many registrations may wait for the tracker before a registering item
/// has to wait for its reply.
const PENDING_REGISTRATIONS: usize = 16;

/// A status notifier item that asked to be tracked.
struct Registration {
    destination: String,
    path: String,
}

pub struct Watcher {
    items: Arc<RwLock<BTreeMap<SystrayId, SystrayItem>>>,
    registrations: mpsc::Sender<Registration>,
    /// Cranky is itself the host, so one is registered from the start; another
    /// host registering leaves it set.
    host_registered: bool,
    protocol_version: i32,
}

impl Watcher {
    /// Builds the watcher and starts the task that tracks every item it is
    /// asked to register, each in a task of its own.
    ///
    /// Must be called from within a tokio runtime.
    #[must_use]
    pub fn new(
        items: Arc<RwLock<BTreeMap<SystrayId, SystrayItem>>>,
        hub: Arc<SignalHub>,
        conn: Connection,
    ) -> Self {
        let (registrations, mut pending) = mpsc::channel::<Registration>(PENDING_REGISTRATIONS);
        let tracked = items.clone();
        tokio::spawn(async move {
            while let Some(registration) = pending.recv().await {
                let (conn, items, hub) = (conn.clone(), tracked.clone(), hub.clone());
                tokio::spawn(async move {
                    let Registration { destination, path } = registration;
                    if let Err(e) = Self::track_item(conn, items, hub, destination, path).await {
                        error!("Failed to track SNI item: {e}");
                    }
                });
            }
        });
        Self {
            items,
            registrations,
            host_registered: true,
            protocol_version: 0,
        }
    }
}

#[interface(name = "org.kde.StatusNotifierWatcher")]
impl Watcher {
    async fn register_status_notifier_item(
        &self,
        service: &str,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) {
        debug!("Registered SNI item: {service}");

        let path = if service.starts_with('/') {
            service.to_string()
        } else {
            "/StatusNotifierItem".to_string()
        };

        let destination = if service.starts_with('/') {
            header
                .sender()
                .map_or_else(|| service.to_string(), |s| s.as_str().to_string())
        } else {
            service.to_string()
        };

        let registration = Registration { destination, path };
        if self.registrations.send(registration).await.is_err() {
            error!("The SNI item tracker has stopped; dropping a registration");
        }
    }

    fn register_status_notifier_host(&mut self, service: &str) {
        debug!("Registered SNI host: {service}");
        self.host_registered = true;
    }

    #[zbus(property)]
    async fn registered_status_notifier_items(&self) -> Vec<String> {
        let items = self.items.read().await;
        items.keys().map(|id| id.as_str().to_string()).collect()
    }

    #[zbus(property)]
    const fn is_status_notifier_host_registered(&self) -> bool {
        self.host_registered
    }

    #[zbus(property)]
    const fn protocol_version(&self) -> i32 {
        self.protocol_version
    }
}
