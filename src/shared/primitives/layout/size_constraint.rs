use super::super::geometry::Rect;

/// The box an embedder resolved for an embedded module's host, per axis.
///
/// `Some(px)` is a definite size the child fills: the host's resolved width,
/// or its height when the embedder's CSS set one. `None` means the child
/// measures that axis intrinsically. The embedder sizes the host from the
/// child's *intrinsic* report (see `ChildReport`), never from the size it was
/// laid out at under this constraint, so the two directions cannot feed back
/// into each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SizeConstraint {
    width: Option<u32>,
    height: Option<u32>,
}

impl SizeConstraint {
    #[must_use]
    pub const fn none() -> Self {
        Self {
            width: None,
            height: None,
        }
    }

    #[must_use]
    pub const fn new(width: Option<u32>, height: Option<u32>) -> Self {
        Self { width, height }
    }

    #[must_use]
    pub const fn width(&self) -> Option<u32> {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> Option<u32> {
        self.height
    }
}

/// A child's rect within its parent's tree, plus whatever size constraint the
/// parent's CSS placed on it. What a `LayoutSender` delivers to the child.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChildBounds {
    rect: Rect,
    constraint: SizeConstraint,
}

impl ChildBounds {
    #[must_use]
    pub const fn new(rect: Rect, constraint: SizeConstraint) -> Self {
        Self { rect, constraint }
    }

    #[must_use]
    pub const fn rect(&self) -> Rect {
        self.rect
    }

    #[must_use]
    pub const fn constraint(&self) -> SizeConstraint {
        self.constraint
    }
}
