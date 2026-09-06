use super::types::MarginOffset;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MarginConfig {
    top: MarginOffset,
    bottom: MarginOffset,
    left: MarginOffset,
    right: MarginOffset,
}

impl MarginConfig {
    #[must_use]
    pub const fn new(
        top: MarginOffset,
        bottom: MarginOffset,
        left: MarginOffset,
        right: MarginOffset,
    ) -> Self {
        Self {
            top,
            bottom,
            left,
            right,
        }
    }

    #[must_use]
    pub const fn top(&self) -> MarginOffset {
        self.top
    }

    #[must_use]
    pub const fn bottom(&self) -> MarginOffset {
        self.bottom
    }

    #[must_use]
    pub const fn left(&self) -> MarginOffset {
        self.left
    }

    #[must_use]
    pub const fn right(&self) -> MarginOffset {
        self.right
    }

    #[must_use]
    pub const fn with_partial_overrides(&self, partial: &PartialMarginConfig) -> Self {
        let top = match partial.top() {
            Some(t) => t,
            None => self.top,
        };
        let bottom = match partial.bottom() {
            Some(b) => b,
            None => self.bottom,
        };
        let left = match partial.left() {
            Some(l) => l,
            None => self.left,
        };
        let right = match partial.right() {
            Some(r) => r,
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
pub struct PartialMarginConfig {
    top: Option<MarginOffset>,
    bottom: Option<MarginOffset>,
    left: Option<MarginOffset>,
    right: Option<MarginOffset>,
}

impl PartialMarginConfig {
    #[must_use]
    pub const fn new(
        top: Option<MarginOffset>,
        bottom: Option<MarginOffset>,
        left: Option<MarginOffset>,
        right: Option<MarginOffset>,
    ) -> Self {
        Self {
            top,
            bottom,
            left,
            right,
        }
    }

    #[must_use]
    pub const fn top(&self) -> Option<MarginOffset> {
        self.top
    }
    #[must_use]
    pub const fn bottom(&self) -> Option<MarginOffset> {
        self.bottom
    }
    #[must_use]
    pub const fn left(&self) -> Option<MarginOffset> {
        self.left
    }
    #[must_use]
    pub const fn right(&self) -> Option<MarginOffset> {
        self.right
    }
}
