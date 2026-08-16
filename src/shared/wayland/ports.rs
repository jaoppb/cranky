use crate::shared::primitives::render::RenderBuffer;
use crate::shared::primitives::{ModuleId, MonitorId};
use crate::shared::primitives::geometry::Position;
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait SurfaceManagerPort: Send + Sync {
    /// Submit a rendered buffer for a specific module on a specific monitor.
    async fn submit_buffer(&self, module_id: ModuleId, monitor_id: MonitorId, position: Position, buffer: RenderBuffer);
}

pub type DynSurfaceManager = Arc<dyn SurfaceManagerPort>;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DisplayServerError {
    #[error("Display server connection failed: {reason}")]
    ConnectionFailed { reason: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Internal error: {0}")]
    Internal(String),
}

#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait DisplayServerPort: Send + Sync {
    async fn wait_for_events(&mut self) -> Result<(), DisplayServerError>;
    fn dispatch_pending(&mut self) -> Result<(), DisplayServerError>;
    fn flush(&mut self) -> Result<(), DisplayServerError>;
    fn render_all(
        &mut self,
        read_model: &crate::app::state::AppReadModel,
        layout_senders: &std::collections::HashMap<
            crate::shared::primitives::ModuleId,
            Box<dyn crate::features::module_runtime::ports::LayoutSender>,
        >,
    ) -> Result<(), DisplayServerError>;
    fn show_tooltip(&mut self, layout: crate::features::layout_engine::domain::LayoutNode) -> Result<(), DisplayServerError>;
    fn hide_tooltip(&mut self) -> Result<(), DisplayServerError>;
}
