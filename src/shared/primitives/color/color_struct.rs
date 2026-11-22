/// An 8-bit-per-channel RGBA color, serialised with the keys `r`, `g`, `b` and `a`.
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
    /// Builds an opaque color from its red, green and blue channels.
    #[must_use]
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 255,
        }
    }

    /// This color with its alpha channel replaced: straight (not premultiplied), 255 is opaque.
    #[must_use]
    pub(crate) const fn with_alpha(mut self, alpha: u8) -> Self {
        self.alpha = alpha;
        self
    }

    #[must_use]
    pub(crate) const fn red(self) -> u8 {
        self.red
    }

    #[must_use]
    pub(crate) const fn green(self) -> u8 {
        self.green
    }

    #[must_use]
    pub(crate) const fn blue(self) -> u8 {
        self.blue
    }

    #[must_use]
    pub(crate) const fn alpha(self) -> u8 {
        self.alpha
    }
}
