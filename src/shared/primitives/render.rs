use crate::shared::primitives::binary::BinaryData;
use crate::shared::primitives::geometry::Size;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderBuffer {
    data: BinaryData,
    size: Size,
}

impl RenderBuffer {
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
