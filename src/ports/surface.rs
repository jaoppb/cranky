use crate::domain::shared::render::RenderBuffer;
use crate::domain::{ModuleId, MonitorId};
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait SurfaceManagerPort: Send + Sync {
    /// Submit a rendered buffer for a specific module on a specific monitor.
    async fn submit_buffer(&self, module_id: ModuleId, monitor_id: MonitorId, buffer: RenderBuffer);
}

pub type DynSurfaceManager = Arc<dyn SurfaceManagerPort>;
