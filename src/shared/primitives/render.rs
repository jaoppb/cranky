use crate::shared::primitives::binary::BinaryData;
use crate::shared::primitives::geometry::Size;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderBuffer {
    data: BinaryData,
    size: Size,
}

impl RenderBuffer {
    pub fn new(data: impl Into<BinaryData>, size: Size) -> Self {
        Self {
            data: data.into(),
            size,
        }
    }

    pub fn size(&self) -> &Size {
        &self.size
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn width(&self) -> u32 {
        self.size.width()
    }

    pub fn height(&self) -> u32 {
        self.size.height()
    }
}
