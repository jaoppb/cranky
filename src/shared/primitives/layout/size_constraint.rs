use super::super::geometry::Rect;

/// A parent's CSS pin on an embedded module's slot, per axis.
///
/// `Some(px)` means the parent set an explicit `width`/`height` and taffy
/// already resolved it to that many pixels for this slot; `None` means the
/// child measures that axis intrinsically. Never built from anything the
/// child reported — only from a static CSS pin — so it cannot reintroduce a
/// parent/child size feedback loop.
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
