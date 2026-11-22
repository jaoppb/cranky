//! A rendered buffer queued for a module's surface on one monitor, waiting for the Wayland loop to
//! attach it.
use crate::shared::primitives::geometry::Position;
use crate::shared::primitives::render::{ModuleFrame, RenderBuffer};
use crate::shared::primitives::{ModuleId, MonitorId};
use crate::shared::wayland::surface_parent::SurfaceParent;

pub(crate) struct SurfaceCommand {
    frame: ModuleFrame,
    parent: Option<SurfaceParent>,
}

impl SurfaceCommand {
    #[must_use]
    pub(crate) const fn new(frame: ModuleFrame, parent: Option<SurfaceParent>) -> Self {
        Self { frame, parent }
    }

    /// The module's frame as painted, for a consumer that keeps it whole (a composite layer).
    #[must_use]
    pub(crate) const fn frame(&self) -> &ModuleFrame {
        &self.frame
    }

    #[must_use]
    pub(crate) const fn module_id(&self) -> ModuleId {
        self.frame.module_id()
    }

    #[must_use]
    pub(crate) const fn parent(&self) -> Option<&SurfaceParent> {
        self.parent.as_ref()
    }

    #[must_use]
    pub(crate) const fn monitor_id(&self) -> &MonitorId {
        self.frame.monitor_id()
    }

    #[must_use]
    pub(crate) const fn position(&self) -> Position {
        self.frame.position()
    }

    #[must_use]
    pub(crate) const fn buffer(&self) -> &RenderBuffer {
        self.frame.buffer()
    }
}
