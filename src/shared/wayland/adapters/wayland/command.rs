use crate::shared::primitives::geometry::Position;
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::primitives::{ModuleId, MonitorId};
use crate::shared::wayland::surface_parent::SurfaceParent;

pub struct SurfaceCommand {
    module_id: ModuleId,
    parent: Option<SurfaceParent>,
    monitor_id: MonitorId,
    position: Position,
    buffer: RenderBuffer,
}

impl SurfaceCommand {
    #[must_use]
    pub const fn new(
        module_id: ModuleId,
        parent: Option<SurfaceParent>,
        monitor_id: MonitorId,
        position: Position,
        buffer: RenderBuffer,
    ) -> Self {
        Self {
            module_id,
            parent,
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
    pub const fn parent(&self) -> Option<&SurfaceParent> {
        self.parent.as_ref()
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
