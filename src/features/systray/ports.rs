//! The tray's ports: the SNI host that modules drive, and the cache for resolved icons.
use async_trait::async_trait;
use thiserror::Error;

/// Failures of the SNI host that its callers can see.
#[derive(Error, Debug)]
pub enum SniPortError {
    /// The session bus could not be reached or the watcher object could not be served.
    #[error("Failed to start SNI watcher: {0}")]
    StartFailed(String),
    /// No D-Bus proxy could be built for the item an action targets.
    #[error("Failed to trigger action on {id}: {error}")]
    ActionFailed {
        /// The [`SystrayId`](crate::features::systray::domain::SystrayId) of the targeted item.
        id: String,
        /// The D-Bus error that prevented the proxy.
        error: String,
    },
}

/// The tray host as the app drives it: start it once, then forward actions from scripts to
/// items.
#[async_trait]
pub trait SniPort: Send + Sync {
    /// Initialize the SNI Host (and optionally the Watcher)
    async fn start(&mut self) -> Result<(), SniPortError>;

    /// Trigger an action on a systray item (e.g. `Activate`, `SecondaryActivate`, `ContextMenu`)
    async fn trigger_action(
        &self,
        id: &crate::features::systray::domain::SystrayId,
        action: &crate::features::systray::domain::SystrayActionName,
        pos: Option<crate::shared::primitives::geometry::Position>,
    ) -> Result<(), SniPortError>;
}

/// A cached icon lookup: the image found, or a lookup that found none and is
/// not retried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CachedIcon {
    Unresolved,
    Image(crate::features::systray::domain::IconImage),
}

impl CachedIcon {
    #[must_use]
    pub(crate) fn into_image(self) -> Option<crate::features::systray::domain::IconImage> {
        match self {
            Self::Unresolved => None,
            Self::Image(image) => Some(image),
        }
    }
}

impl From<Option<crate::features::systray::domain::IconImage>> for CachedIcon {
    fn from(image: Option<crate::features::systray::domain::IconImage>) -> Self {
        image.map_or(Self::Unresolved, Self::Image)
    }
}

pub(crate) trait SystrayIconCachePort: Send + Sync {
    fn get(&self, key: &crate::features::systray::domain::IconCacheKey) -> Option<CachedIcon>;
    fn insert(&self, key: crate::features::systray::domain::IconCacheKey, icon: CachedIcon);
}
