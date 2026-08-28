use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SniPortError {
    #[error("Failed to start SNI watcher: {0}")]
    StartFailed(String),
    #[error("Failed to trigger action on {id}: {error}")]
    ActionFailed { id: String, error: String },
}

#[cfg_attr(test, mockall::automock)]
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

#[cfg_attr(test, mockall::automock)]
pub trait SystrayIconCachePort: Send + Sync {
    fn get(
        &self,
        key: &crate::features::systray::domain::IconCacheKey,
    ) -> Option<Option<crate::features::systray::domain::IconImage>>;
    fn insert(
        &self,
        key: crate::features::systray::domain::IconCacheKey,
        image: Option<crate::features::systray::domain::IconImage>,
    );
}
