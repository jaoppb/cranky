use crate::shared::primitives::geometry::Position;
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::primitives::{ModuleId, MonitorId};
use async_trait::async_trait;
use std::sync::Arc;

#[cfg_attr(test, mockall::automock)]
pub trait SurfaceManagerPort: Send + Sync {
    /// Submit a rendered buffer for a specific module on a specific monitor.
    fn submit_buffer(
        &self,
        module_id: ModuleId,
        monitor_id: MonitorId,
        position: Position,
        buffer: RenderBuffer,
    );

    /// Submit a rendered buffer with optional parent module ID for nested subsurfaces.
    fn submit_child_buffer(
        &self,
        module_id: ModuleId,
        parent_id: Option<ModuleId>,
        monitor_id: MonitorId,
        position: Position,
        buffer: RenderBuffer,
    ) {
        let _ = parent_id;
        self.submit_buffer(module_id, monitor_id, position, buffer);
    }
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
    /// # Errors
    ///
    /// Returns `DisplayServerError` if waiting for display events fails.
    async fn wait_for_events(&mut self) -> Result<(), DisplayServerError>;

    /// # Errors
    ///
    /// Returns `DisplayServerError` if dispatching pending events fails.
    fn dispatch_pending(&mut self) -> Result<(), DisplayServerError>;

    /// # Errors
    ///
    /// Returns `DisplayServerError` if flushing requests to the display server fails.
    fn flush(&mut self) -> Result<(), DisplayServerError>;

    /// # Errors
    ///
    /// Returns `DisplayServerError` if rendering fails.
    fn render_all(
        &mut self,
        read_model: &crate::app::state::AppReadModel,
        layout_senders: &std::collections::HashMap<
            crate::shared::primitives::ModuleId,
            Box<dyn crate::features::module_runtime::ports::LayoutSender>,
        >,
    ) -> Result<(), DisplayServerError>;

    /// # Errors
    ///
    /// Returns `DisplayServerError` if displaying the tooltip fails.
    fn show_tooltip(
        &mut self,
        layout: crate::features::layout_engine::domain::StyledNode,
    ) -> Result<(), DisplayServerError>;

    /// # Errors
    ///
    /// Returns `DisplayServerError` if hiding the tooltip fails.
    fn hide_tooltip(&mut self) -> Result<(), DisplayServerError>;
}
