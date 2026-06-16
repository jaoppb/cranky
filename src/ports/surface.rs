use crate::domain::{ModuleId, MonitorId};
use crate::domain::render::RenderBuffer;
use std::sync::Arc;
use async_trait::async_trait;

#[async_trait]
pub trait SurfaceManagerPort: Send + Sync {
    /// Submit a rendered buffer for a specific module on a specific monitor.
    async fn submit_buffer(&self, module_id: ModuleId, monitor_id: MonitorId, buffer: RenderBuffer);
}

pub type DynSurfaceManager = Arc<dyn SurfaceManagerPort>;
