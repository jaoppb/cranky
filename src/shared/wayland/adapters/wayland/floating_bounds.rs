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
    pub(super) w: i32,
    pub(super) h: i32,
}

/// `precise` requests the exact rect (used by panels, and by any popup
/// nested inside another floating surface — decision 7's "translate a
/// child's anchor rect from its subsurface coordinates into the parent
/// popup's space") over the bar-popup convention of a fixed-height column
/// spanning the whole bar regardless of click position.
pub(super) fn compute_module_bounds(
    placement: Option<ChildPlacement>,
    anchor_rect: Option<Rect>,
    default_h: i32,
    precise: bool,
) -> ModuleAnchorBounds {
    match (placement, anchor_rect) {
        (Some(ms), Some(r)) => {
            let y = if precise { ms.y.saturating_add(r.y()) } else { 0 };
            let h = if precise { to_i32(r.height()) } else { default_h };
            ModuleAnchorBounds {
                x: ms.x.saturating_add(r.x()),
                y,
                w: to_i32(r.width()),
                h,
            }
        }
        (Some(ms), None) => {
            let y = if precise { ms.y } else { 0 };
            let h = if precise { to_i32(ms.size.height()) } else { default_h };
            ModuleAnchorBounds {
                x: ms.x,
                y,
                w: to_i32(ms.size.width()),
                h,
            }
        }
        (None, Some(r)) => {
            let y = if precise { r.y() } else { 0 };
            let h = if precise { to_i32(r.height()) } else { default_h };
            ModuleAnchorBounds {
                x: r.x(),
                y,
                w: to_i32(r.width()),
                h,
            }
        }
        (None, None) => ModuleAnchorBounds {
            x: 0,
            y: 0,
            w: 1,
            h: default_h,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{ChildPlacement, compute_module_bounds, to_i32};
    use crate::shared::primitives::geometry::{Position, Rect, Size};

    #[test]
    fn precise_bounds_offset_anchor_rect_by_child_origin() {
        let placement = ChildPlacement::new(30, 10, Size::new(4, 4));
        let rect = Rect::new(Position::new(2, 3), Size::new(5, 6));
        let b = compute_module_bounds(Some(placement), Some(rect), 24, true);
        assert_eq!((b.x, b.y, b.w, b.h), (32, 13, 5, 6));
        let b = compute_module_bounds(Some(placement), None, 24, true);
        assert_eq!((b.x, b.y, b.w, b.h), (30, 10, 4, 4));
    }

    #[test]
    fn bar_bounds_span_bar_height_from_top() {
        let placement = ChildPlacement::new(30, 10, Size::new(4, 4));
        let b = compute_module_bounds(Some(placement), None, 24, false);
        assert_eq!((b.x, b.y, b.w, b.h), (30, 0, 4, 24));
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
