use async_trait::async_trait;
use crate::adapters::sni::SniAdapterError;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SniPort: Send + Sync {
    /// Initialize the SNI Host (and optionally the Watcher)
    async fn start(&mut self) -> Result<(), SniAdapterError>;
    
    /// Trigger an action on an applet (e.g. "Activate", "SecondaryActivate", "ContextMenu")
    async fn trigger_action(&self, id: &str, action: &str) -> Result<(), SniAdapterError>;
}
