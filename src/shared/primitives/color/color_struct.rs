#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Color {
    #[serde(rename = "r")]
    red: u8,
    #[serde(rename = "g")]
    green: u8,
    #[serde(rename = "b")]
    blue: u8,
    #[serde(rename = "a")]
    alpha: u8,
}

impl Color {
    #[must_use]
    pub const fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    #[must_use]
    pub const fn red(&self) -> u8 {
        self.red
    }

    #[must_use]
    pub const fn green(&self) -> u8 {
        self.green
    }

    #[must_use]
    pub const fn blue(&self) -> u8 {
        self.blue
    }

    #[must_use]
    pub const fn alpha(&self) -> u8 {
        self.alpha
    }
}
