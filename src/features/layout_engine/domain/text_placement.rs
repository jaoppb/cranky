use crate::features::styling::domain::ComputedStyle;
use crate::shared::primitives::TextAlignment;
use crate::shared::primitives::geometry::{Position, Rect};
use crate::utils::f64_to_i64;

/// A padding edge as whole pixels; negative padding counts as none.
fn inset(v: f64) -> u32 {
    u32::try_from(f64_to_i64(v.max(0.0))).unwrap_or(0)
}

/// Where a text leaf's glyphs start: the box's origin moved in by its
/// padding, which layout already added to the box (see `leaf_size`).
pub(super) fn text_origin(rect: &Rect, style: &ComputedStyle) -> Position {
    let signed = |v: f64| i32::try_from(inset(v)).unwrap_or(0);
    let (left, top) = style
        .padding()
        .map_or((0, 0), |p| (signed(p.left()), signed(p.top())));
    Position::new(rect.x().saturating_add(left), rect.y().saturating_add(top))
}

/// The width left for glyphs once layout's padding is taken off the box;
/// what `text-align` aligns within.
pub(super) fn text_alignment(rect: &Rect, style: &ComputedStyle) -> TextAlignment {
    let padding = style
        .padding()
        .map_or(0, |p| inset(p.left()).saturating_add(inset(p.right())));
    TextAlignment::new(
        style.text_align().unwrap_or_default(),
        rect.width().saturating_sub(padding),
    )
}
