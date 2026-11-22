//! The fixture's floating-surface show requests.

use crate::features::layout_engine::domain::{FloatingKind, ShowFloating};
use crate::shared::primitives::render::RenderBuffer;

/// A request to show `buffer` as `kind`, nested in `parent`, at the buffer's
/// own size, with no monitor, no anchor rect and no offset of its own.
pub(in crate::shared::wayland::adapters::wayland) fn show_request(
    kind: FloatingKind,
    buffer: RenderBuffer,
    parent: Option<FloatingKind>,
) -> ShowFloating {
    ShowFloating {
        kind,
        monitor_id: None,
        anchor_rect: None,
        logical_size: *buffer.size(),
        buffer,
        offset: None,
        parent,
    }
}
