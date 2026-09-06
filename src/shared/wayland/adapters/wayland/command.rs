use crate::shared::primitives::geometry::Position;
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::primitives::{ModuleId, MonitorId};

pub struct SurfaceCommand {
    module_id: ModuleId,
    parent_id: Option<ModuleId>,
    monitor_id: MonitorId,
    position: Position,
    buffer: RenderBuffer,
}

impl SurfaceCommand {
    #[must_use]
    pub fn new(
        module_id: ModuleId,
        parent_id: Option<ModuleId>,
        monitor_id: MonitorId,
        position: Position,
        buffer: RenderBuffer,
    ) -> Self {
        Self {
            module_id,
            parent_id,
            monitor_id,
            position,
            buffer,
        }
    }

    #[must_use]
    pub const fn module_id(&self) -> ModuleId {
        self.module_id
    }

    #[must_use]
    pub const fn parent_id(&self) -> Option<ModuleId> {
        self.parent_id
    }

    #[must_use]
    pub const fn monitor_id(&self) -> &MonitorId {
        &self.monitor_id
    }

    #[must_use]
    pub const fn position(&self) -> Position {
        self.position
    }

    #[must_use]
    pub const fn buffer(&self) -> &RenderBuffer {
        &self.buffer
    }
}
