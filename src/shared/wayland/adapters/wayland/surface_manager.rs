use super::command::SurfaceCommand;
use crate::shared::primitives::render::ModuleFrame;
use crate::shared::primitives::{ModuleId, MonitorId};
use crate::shared::wayland::ports::{SurfaceManagerPort, SurfaceParent};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// The cloneable handle render pipelines submit buffers through.
///
/// It keeps only the latest buffer per module and monitor, and wakes the Wayland loop after each
/// submission.
#[derive(Clone)]
pub struct WaylandSurfaceManager {
    pub(crate) pending_surfaces: Arc<Mutex<HashMap<(ModuleId, MonitorId), SurfaceCommand>>>,
    pub(crate) notify_tx: tokio::sync::mpsc::Sender<()>,
}

impl std::fmt::Debug for WaylandSurfaceManager {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WaylandSurfaceManager")
            .finish_non_exhaustive()
    }
}

impl SurfaceManagerPort for WaylandSurfaceManager {
    fn submit_buffer(&self, frame: ModuleFrame) {
        self.submit_child_buffer(frame, None);
    }

    fn submit_child_buffer(&self, frame: ModuleFrame, parent: Option<SurfaceParent>) {
        {
            let mut map = self
                .pending_surfaces
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            map.insert(
                (frame.module_id(), frame.monitor_id().clone()),
                SurfaceCommand::new(frame, parent),
            );
        }
        let _ = self.notify_tx.try_send(());
    }
}
