//! The vertical and horizontal edge pairs a panel can be anchored to, and the bit each edge takes
//! in a `PanelAnchor`.

pub(super) const TOP_FLAG: u8 = 1 << 0;
pub(super) const BOTTOM_FLAG: u8 = 1 << 1;
pub(super) const LEFT_FLAG: u8 = 1 << 2;
pub(super) const RIGHT_FLAG: u8 = 1 << 3;

/// Vertical edge anchor for floating panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum VerticalAnchor {
    #[default]
    None,
    Top,
    Bottom,
    Both,
}

impl VerticalAnchor {
    /// Anchors the top edge as well, keeping the bottom one if it was anchored.
    #[must_use]
    pub(crate) const fn with_top(self) -> Self {
        match self {
            Self::None | Self::Top => Self::Top,
            Self::Bottom | Self::Both => Self::Both,
        }
    }

    /// Anchors the bottom edge as well, keeping the top one if it was anchored.
    #[must_use]
    pub(crate) const fn with_bottom(self) -> Self {
        match self {
            Self::None | Self::Bottom => Self::Bottom,
            Self::Top | Self::Both => Self::Both,
        }
    }

    #[must_use]
    pub(crate) const fn to_flags(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Top => TOP_FLAG,
            Self::Bottom => BOTTOM_FLAG,
            Self::Both => TOP_FLAG | BOTTOM_FLAG,
        }
    }
}

/// Horizontal edge anchor for floating panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum HorizontalAnchor {
    #[default]
    None,
    Left,
    Right,
    Both,
}

impl HorizontalAnchor {
    /// Anchors the left edge as well, keeping the right one if it was anchored.
    #[must_use]
    pub(crate) const fn with_left(self) -> Self {
        match self {
            Self::None | Self::Left => Self::Left,
            Self::Right | Self::Both => Self::Both,
        }
    }

    /// Anchors the right edge as well, keeping the left one if it was anchored.
    #[must_use]
    pub(crate) const fn with_right(self) -> Self {
        match self {
            Self::None | Self::Right => Self::Right,
            Self::Left | Self::Both => Self::Both,
        }
    }

    #[must_use]
    pub(crate) const fn to_flags(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Left => LEFT_FLAG,
            Self::Right => RIGHT_FLAG,
            Self::Both => LEFT_FLAG | RIGHT_FLAG,
        }
    }
}
