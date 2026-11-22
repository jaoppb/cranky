//! Margins of the bar's layer surface, and the partial overrides the unfocused state applies on
//! top.
use super::types::MarginOffset;

/// Gap between the bar and each screen edge, applied as the layer surface's margins.
/// The top and bottom margins are added to the bar height to form its exclusive zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct MarginConfig {
    top: MarginOffset,
    bottom: MarginOffset,
    left: MarginOffset,
    right: MarginOffset,
}

/// The margins at the two ends of one axis: top and bottom, or left and right, as the
/// `vertical` and `horizontal` keys of a margin table set them together and a box's padding or
/// margin is built by axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AxisMargins<T> {
    start: T,
    end: T,
}

impl<T> AxisMargins<T> {
    /// Pairs the margin at the start of the axis (top or left) with the one at its end (bottom or
    /// right).
    #[must_use]
    pub const fn new(start: T, end: T) -> Self {
        Self { start, end }
    }
}

impl<T: Copy> AxisMargins<T> {
    /// The same margin at both ends.
    #[must_use]
    pub(crate) const fn both(margin: T) -> Self {
        Self::new(margin, margin)
    }

    #[must_use]
    pub(crate) const fn start(&self) -> T {
        self.start
    }

    #[must_use]
    pub(crate) const fn end(&self) -> T {
        self.end
    }
}

impl MarginConfig {
    #[must_use]
    pub(crate) const fn new(
        vertical: AxisMargins<MarginOffset>,
        horizontal: AxisMargins<MarginOffset>,
    ) -> Self {
        Self {
            top: vertical.start,
            bottom: vertical.end,
            left: horizontal.start,
            right: horizontal.end,
        }
    }

    #[must_use]
    pub(crate) const fn top(&self) -> MarginOffset {
        self.top
    }

    #[must_use]
    pub(crate) const fn bottom(&self) -> MarginOffset {
        self.bottom
    }

    #[must_use]
    pub(crate) const fn left(&self) -> MarginOffset {
        self.left
    }

    #[must_use]
    pub(crate) const fn right(&self) -> MarginOffset {
        self.right
    }

    #[must_use]
    pub(crate) const fn with_partial_overrides(&self, partial: &PartialMarginConfig) -> Self {
        let top = match partial.top() {
            Some(offset) => offset,
            None => self.top,
        };
        let bottom = match partial.bottom() {
            Some(offset) => offset,
            None => self.bottom,
        };
        let left = match partial.left() {
            Some(offset) => offset,
            None => self.left,
        };
        let right = match partial.right() {
            Some(offset) => offset,
            None => self.right,
        };
        Self {
            top,
            bottom,
            left,
            right,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PartialMarginConfig {
    top: Option<MarginOffset>,
    bottom: Option<MarginOffset>,
    left: Option<MarginOffset>,
    right: Option<MarginOffset>,
}

impl PartialMarginConfig {
    #[must_use]
    pub(crate) const fn new(
        vertical: AxisMargins<Option<MarginOffset>>,
        horizontal: AxisMargins<Option<MarginOffset>>,
    ) -> Self {
        Self {
            top: vertical.start,
            bottom: vertical.end,
            left: horizontal.start,
            right: horizontal.end,
        }
    }

    #[must_use]
    pub(crate) const fn top(&self) -> Option<MarginOffset> {
        self.top
    }
    #[must_use]
    pub(crate) const fn bottom(&self) -> Option<MarginOffset> {
        self.bottom
    }
    #[must_use]
    pub(crate) const fn left(&self) -> Option<MarginOffset> {
        self.left
    }
    #[must_use]
    pub(crate) const fn right(&self) -> Option<MarginOffset> {
        self.right
    }
}
