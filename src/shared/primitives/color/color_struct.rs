#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Color {
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    #[must_use]
    pub const fn r(&self) -> u8 {
        self.r
    }

    #[must_use]
    pub const fn g(&self) -> u8 {
        self.g
    }

    #[must_use]
    pub const fn b(&self) -> u8 {
        self.b
    }

    #[must_use]
    pub const fn a(&self) -> u8 {
        self.a
    }
}
