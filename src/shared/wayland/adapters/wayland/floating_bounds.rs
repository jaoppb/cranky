use crate::shared::primitives::geometry::{Rect, Size};

pub(super) fn to_i32(val: u32) -> i32 {
    i32::try_from(val).unwrap_or(i32::MAX)
}

/// Where an embedded child sits inside its parent (a bar or a floating
/// surface), whether on its own subsurface or composited as a layer: its
/// logical origin, and its buffer size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ChildPlacement {
    x: i32,
    y: i32,
    size: Size,
}

impl ChildPlacement {
    pub(super) const fn new(x: i32, y: i32, size: Size) -> Self {
        Self { x, y, size }
    }
}

pub(super) struct ModuleAnchorBounds {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) width: i32,
    pub(super) height: i32,
}

/// How much of its parent a floating surface's anchor covers vertically.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AnchorExtent {
    /// The exact rect: used by panels, and by any popup nested inside another
    /// floating surface (decision 7's "translate a child's anchor rect from its
    /// subsurface coordinates into the parent popup's space").
    Exact,
    /// The bar-popup convention: a fixed-height column spanning the whole bar
    /// regardless of click position.
    BarColumn,
}

impl ModuleAnchorBounds {
    /// These bounds at `extent`: unchanged when exact, else stretched to the
    /// whole `bar_height` column from the top.
    pub(super) const fn spanning(self, extent: AnchorExtent, bar_height: i32) -> Self {
        match extent {
            AnchorExtent::Exact => self,
            AnchorExtent::BarColumn => Self {
                y: 0,
                height: bar_height,
                ..self
            },
        }
    }
}

/// Where a floating surface anchors on its parent, exactly: the child's
/// placement offset by `anchor_rect`, either alone, or a thin `default_h`
/// column at the origin without both.
pub(super) fn compute_module_bounds(
    placement: Option<ChildPlacement>,
    anchor_rect: Option<Rect>,
    default_h: i32,
) -> ModuleAnchorBounds {
    match (placement, anchor_rect) {
        (Some(child), Some(rect)) => ModuleAnchorBounds::of_rect(rect).offset_by(child),
        (Some(child), None) => ModuleAnchorBounds::of_child(child),
        (None, Some(rect)) => ModuleAnchorBounds::of_rect(rect),
        (None, None) => ModuleAnchorBounds {
            x: 0,
            y: 0,
            width: 1,
            height: default_h,
        },
    }
}

impl ModuleAnchorBounds {
    /// Exactly `rect`.
    fn of_rect(rect: Rect) -> Self {
        Self {
            x: rect.x(),
            y: rect.y(),
            width: to_i32(rect.width()),
            height: to_i32(rect.height()),
        }
    }

    /// The whole of `child`: its origin and its buffer size.
    fn of_child(child: ChildPlacement) -> Self {
        Self {
            x: child.x,
            y: child.y,
            width: to_i32(child.size.width()),
            height: to_i32(child.size.height()),
        }
    }

    /// These bounds moved by `child`'s origin.
    const fn offset_by(self, child: ChildPlacement) -> Self {
        Self {
            x: child.x.saturating_add(self.x),
            y: child.y.saturating_add(self.y),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AnchorExtent, ChildPlacement, compute_module_bounds, to_i32};
    use crate::shared::primitives::geometry::{Position, Rect, Size};

    #[test]
    fn precise_bounds_offset_anchor_rect_by_child_origin() {
        let placement = ChildPlacement::new(30, 10, Size::new(4, 4));
        let rect = Rect::new(Position::new(2, 3), Size::new(5, 6));
        let bounds = compute_module_bounds(Some(placement), Some(rect), 24)
            .spanning(AnchorExtent::Exact, 24);
        assert_eq!(
            (bounds.x, bounds.y, bounds.width, bounds.height),
            (32, 13, 5, 6)
        );
        let bounds =
            compute_module_bounds(Some(placement), None, 24).spanning(AnchorExtent::Exact, 24);
        assert_eq!(
            (bounds.x, bounds.y, bounds.width, bounds.height),
            (30, 10, 4, 4)
        );
    }

    #[test]
    fn bar_bounds_span_bar_height_from_top() {
        let placement = ChildPlacement::new(30, 10, Size::new(4, 4));
        let bounds =
            compute_module_bounds(Some(placement), None, 24).spanning(AnchorExtent::BarColumn, 24);
        assert_eq!(
            (bounds.x, bounds.y, bounds.width, bounds.height),
            (30, 0, 4, 24)
        );
    }

    #[test]
    fn bar_bounds_with_anchor_rect_keep_rect_x_and_span_bar_height() {
        let placement = ChildPlacement::new(30, 10, Size::new(4, 4));
        let rect = Rect::new(Position::new(2, 3), Size::new(5, 6));
        let bounds = compute_module_bounds(Some(placement), Some(rect), 24)
            .spanning(AnchorExtent::BarColumn, 24);
        assert_eq!(
            (bounds.x, bounds.y, bounds.width, bounds.height),
            (32, 0, 5, 24)
        );
    }

    #[test]
    fn bounds_without_placement_use_the_anchor_rect_alone() {
        let rect = Rect::new(Position::new(2, 3), Size::new(5, 6));
        let bounds = compute_module_bounds(None, Some(rect), 24).spanning(AnchorExtent::Exact, 24);
        assert_eq!(
            (bounds.x, bounds.y, bounds.width, bounds.height),
            (2, 3, 5, 6)
        );
        let bounds =
            compute_module_bounds(None, Some(rect), 24).spanning(AnchorExtent::BarColumn, 24);
        assert_eq!(
            (bounds.x, bounds.y, bounds.width, bounds.height),
            (2, 0, 5, 24)
        );
    }

    #[test]
    fn bounds_without_placement_or_rect_fall_back_to_a_thin_bar_column() {
        for extent in [AnchorExtent::Exact, AnchorExtent::BarColumn] {
            let bounds = compute_module_bounds(None, None, 24).spanning(extent, 24);
            assert_eq!(
                (bounds.x, bounds.y, bounds.width, bounds.height),
                (0, 0, 1, 24)
            );
        }
    }

    #[test]
    fn to_i32_overflow_falls_back_to_i32_max() {
        let overflowing = u32::MAX;
        assert_eq!(to_i32(overflowing), i32::MAX);
    }

    #[test]
    fn to_i32_in_range_value_round_trips() {
        assert_eq!(to_i32(42), 42);
    }
}
