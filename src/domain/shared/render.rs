use crate::domain::shared::geometry::Size;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderBuffer {
    data: Vec<u8>,
    size: Size,
}

impl RenderBuffer {
    pub fn new(data: Vec<u8>, size: Size) -> Self {
        Self { data, size }
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    pub fn size(&self) -> &Size {
        &self.size
    }

    pub fn width(&self) -> u32 {
        self.size.width()
    }

    pub fn height(&self) -> u32 {
        self.size.height()
    }
}
