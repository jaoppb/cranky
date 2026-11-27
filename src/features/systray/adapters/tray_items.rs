//! The tray's tracked items, shared by every task that follows an item, and the hub their state
//! is published on.
use crate::features::systray::domain::{SystrayId, SystrayItem, SystrayState};
use crate::shared::events::signals::SignalHub;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// The registry of tracked items together with the hub that publishes it: whoever changes the
/// items publishes the new state through the same value.
#[derive(Debug, Clone)]
pub(super) struct TrayItems {
    items: Arc<RwLock<BTreeMap<SystrayId, SystrayItem>>>,
    hub: Arc<SignalHub>,
}

impl TrayItems {
    #[must_use]
    pub(super) const fn new(
        items: Arc<RwLock<BTreeMap<SystrayId, SystrayItem>>>,
        hub: Arc<SignalHub>,
    ) -> Self {
        Self { items, hub }
    }

    #[must_use]
    pub(super) const fn items(&self) -> &Arc<RwLock<BTreeMap<SystrayId, SystrayItem>>> {
        &self.items
    }

    /// Sends a snapshot of the tracked items on the hub's systray channel.
    pub(super) async fn publish(&self) {
        let state = {
            let lock = self.items.read().await;
            SystrayState::new(lock.clone())
        };
        let _ = self.hub.systray_tx().send(state);
    }

    /// Stores `item` under `id`, then publishes the new state once the lock is released.
    pub(super) async fn insert_and_publish(&self, id: SystrayId, item: SystrayItem) {
        {
            let mut lock = self.items.write().await;
            lock.insert(id, item);
        }
        self.publish().await;
    }

    /// Drops the item under `id`, then publishes the new state once the lock is released.
    pub(super) async fn remove_and_publish(&self, id: &SystrayId) {
        {
            let mut lock = self.items.write().await;
            lock.remove(id);
        }
        self.publish().await;
    }
}
