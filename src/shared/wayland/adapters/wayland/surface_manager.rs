use super::command::SurfaceCommand;
use crate::shared::primitives::geometry::Position;
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::primitives::{ModuleId, MonitorId};
use crate::shared::wayland::ports::SurfaceManagerPort;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct WaylandSurfaceManager {
    pub(crate) pending_surfaces: Arc<Mutex<HashMap<(ModuleId, MonitorId), SurfaceCommand>>>,
    pub(crate) notify_tx: tokio::sync::mpsc::Sender<()>,
}

impl SurfaceManagerPort for WaylandSurfaceManager {
    fn submit_buffer(
        &self,
        module_id: ModuleId,
        monitor_id: MonitorId,
        position: Position,
        buffer: RenderBuffer,
    ) {
        self.submit_child_buffer(module_id, None, monitor_id, position, buffer);
    }

    fn submit_child_buffer(
        &self,
        module_id: ModuleId,
        parent_id: Option<ModuleId>,
        monitor_id: MonitorId,
        position: Position,
        buffer: RenderBuffer,
    ) {
        {
            let mut map = self
                .pending_surfaces
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            map.insert(
                (module_id, monitor_id.clone()),
                SurfaceCommand::new(module_id, parent_id, monitor_id, position, buffer),
            );
        }
        let _ = self.notify_tx.try_send(());
    }
}
