//! The painted pixel buffer a render pipeline hands to the surface manager, and the frame that
//! places it on a module's surface.
use crate::shared::primitives::binary::BinaryData;
use crate::shared::primitives::geometry::{Position, Size};
use crate::shared::primitives::{ModuleId, MonitorId};

/// Pixels painted for one module on one monitor, sized in physical pixels, ready to copy into a shm
/// buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderBuffer {
    data: BinaryData,
    size: Size,
}

impl RenderBuffer {
    /// Pairs painted bytes with their size; the byte length is not checked against it.
    #[must_use]
    pub fn new(data: impl Into<BinaryData>, size: Size) -> Self {
        Self {
            data: data.into(),
            size,
        }
    }

    #[must_use]
    pub(crate) const fn size(&self) -> &Size {
        &self.size
    }

    #[must_use]
    pub(crate) fn data(&self) -> &[u8] {
        self.data.as_slice()
    }

    #[must_use]
    pub(crate) const fn width(&self) -> u32 {
        self.size.width()
    }

    #[must_use]
    pub(crate) const fn height(&self) -> u32 {
        self.size.height()
    }
}

/// A painted buffer and where its top-left corner goes on the surface it is shown on, in logical
/// pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedBuffer {
    position: Position,
    buffer: RenderBuffer,
}

impl PlacedBuffer {
    /// Places `buffer` with its top-left corner at `position`.
    #[must_use]
    pub const fn new(position: Position, buffer: RenderBuffer) -> Self {
        Self { position, buffer }
    }
}

/// One frame a module painted for one monitor, with where it goes: the surface manager keeps the
/// latest one per module and monitor, and a popup composites one per embedded child.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleFrame {
    module_id: ModuleId,
    monitor_id: MonitorId,
    placed: PlacedBuffer,
}

impl ModuleFrame {
    /// The frame `module_id` painted for `monitor_id`.
    #[must_use]
    pub const fn new(module_id: ModuleId, monitor_id: MonitorId, placed: PlacedBuffer) -> Self {
        Self {
            module_id,
            monitor_id,
            placed,
        }
    }

    /// The module that painted this frame: its frames replace each other under this id.
    #[must_use]
    pub(crate) const fn module_id(&self) -> ModuleId {
        self.module_id
    }

    /// The monitor this frame was painted for, whose bar it is shown on.
    #[must_use]
    pub(crate) const fn monitor_id(&self) -> &MonitorId {
        &self.monitor_id
    }

    /// Where the frame's top-left corner goes, in logical pixels of its parent surface.
    #[must_use]
    pub(crate) const fn position(&self) -> Position {
        self.placed.position
    }

    /// The pixels the module painted, at their physical (scaled) size.
    #[must_use]
    pub(crate) const fn buffer(&self) -> &RenderBuffer {
        &self.placed.buffer
    }
}
