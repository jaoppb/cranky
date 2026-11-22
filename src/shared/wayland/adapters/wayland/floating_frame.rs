//! What one show of a floating surface hands to Wayland once its anchor is resolved.
use super::floating_anchor::AnchorInfo;
use crate::features::layout_engine::domain::FloatingKind;
use crate::shared::primitives::PopupOffset;
use crate::shared::primitives::render::RenderBuffer;

/// Everything the positioner needs to place a floating surface — no rendering
/// information, since the module already painted its buffer, whose own
/// physical (scaled) size the surface takes. `anchor` is where and on what it
/// is anchored; `text_w`/`text_h` are the logical size the positioner works
/// in.
pub(super) struct FloatingPlacement {
    pub(super) anchor: AnchorInfo,
    pub(super) text_w: i32,
    pub(super) text_h: i32,
    pub(super) offset: PopupOffset,
}

/// One frame of a floating surface, ready for Wayland: which surface it is
/// and the one it nests in (decision 7), the pixels its owner painted, and
/// where they go.
pub(super) struct FloatingFrame {
    pub(super) kind: FloatingKind,
    pub(super) parent: Option<FloatingKind>,
    pub(super) buffer: RenderBuffer,
    pub(super) placement: FloatingPlacement,
}
