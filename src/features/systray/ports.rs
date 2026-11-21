use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SniPortError {
    #[error("Failed to start SNI watcher: {0}")]
    StartFailed(String),
    #[error("Failed to trigger action on {id}: {error}")]
    ActionFailed { id: String, error: String },
}

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
    fn insert(
        &self,
        key: crate::features::systray::domain::IconCacheKey,
        image: Option<crate::features::systray::domain::IconImage>,
    );
}
