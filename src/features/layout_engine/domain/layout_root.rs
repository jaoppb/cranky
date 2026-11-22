//! The tree a layout pass lays out and the box it is laid out in.
use super::styled_node::StyledNode;
use crate::shared::primitives::SizeConstraint;
use crate::shared::primitives::geometry::Position;

/// A styled tree and the box it goes in: the origin every laid-out rect is offset by, and the
/// axes the embedder pins (an unpinned axis is measured intrinsically).
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutRoot {
    node: StyledNode,
    origin: Position,
    constraint: SizeConstraint,
}

impl LayoutRoot {
    /// `node` laid out from `origin`, its root pinned on the axes `constraint` sets.
    #[must_use]
    pub const fn new(node: StyledNode, origin: Position, constraint: SizeConstraint) -> Self {
        Self {
            node,
            origin,
            constraint,
        }
    }

    #[must_use]
    pub(crate) const fn node(&self) -> &StyledNode {
        &self.node
    }

    #[must_use]
    pub(crate) const fn origin(&self) -> Position {
        self.origin
    }

    #[must_use]
    pub(crate) const fn constraint(&self) -> SizeConstraint {
        self.constraint
    }
}
