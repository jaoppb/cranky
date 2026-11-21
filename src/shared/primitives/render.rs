//! The painted pixel buffer a render pipeline hands to the surface manager.
use crate::shared::primitives::binary::BinaryData;
use crate::shared::primitives::geometry::Size;

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
