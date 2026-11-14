use crate::features::layout_engine::domain::{FloatingKind, PopupTarget};
use crate::shared::config::domain::PopupEmbed;
use crate::shared::primitives::geometry::{Position, Rect, Size};
use crate::shared::primitives::{ModuleId, MonitorId};
use crate::shared::wayland::adapters::wayland::floating_anchor::resolve_anchor;
use crate::shared::wayland::adapters::wayland::state::WaylandState;
use crate::shared::wayland::adapters::wayland::tests::fixture::pointer::CHILD;
use crate::shared::wayland::adapters::wayland::tests::fixture::{Fake, popup_kind, solid};

/// A 50×20 popup owned by module 1 on DP-1 with module 9 composited as a
/// 4×4 layer at (30, 10).
fn popup_with_layer_at_30_10(fake: &Fake) -> WaylandState {
    let mut state = fake.state(PopupEmbed::Composite);
    state.bars.push(fake.bar("DP-1", 1));
    let mut floating = fake.floating(Size::new(50, 20), solid(50, 20, [0; 4]));
    let (id, monitor) = (ModuleId::new(CHILD), MonitorId::new("DP-1"));
    let (origin, buffer) = (Position::new(30, 10), solid(4, 4, [1; 4]));
    floating.layers.upsert(id, monitor, origin, buffer);
    state.floating_surfaces.insert(popup_kind(), floating);
    state
}

#[test]
fn nested_popup_anchor_uses_composite_child_origin() {
    let fake = Fake::new();
    let state = popup_with_layer_at_30_10(&fake);
    let target = PopupTarget::new(ModuleId::new(CHILD), MonitorId::new("DP-1"));
    let (kind, parent) = (FloatingKind::Popup(target), popup_kind());
    let bounds = |rect| {
        resolve_anchor(&state, &kind, None, rect, Some(&parent))
            .map(|a| (a.anchor_x, a.anchor_y, a.anchor_w, a.anchor_h))
    };
    let rect = Rect::new(Position::new(2, 3), Size::new(5, 6));
    assert_eq!(bounds(Some(rect)), Some((32, 13, 5, 6)));
    assert_eq!(bounds(None), Some((30, 10, 4, 4)));
}
