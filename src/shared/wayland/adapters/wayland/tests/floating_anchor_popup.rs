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
        resolve_anchor(&state, &kind, None, rect, Some(&parent)).map(|anchor| {
            (
                anchor.anchor_x,
                anchor.anchor_y,
                anchor.anchor_w,
                anchor.anchor_h,
            )
        })
    };
    let rect = Rect::new(Position::new(2, 3), Size::new(5, 6));
    assert_eq!(bounds(Some(rect)), Some((32, 13, 5, 6)));
    assert_eq!(bounds(None), Some((30, 10, 4, 4)));
}

/// The anchor rect a top-level popup on DP-1 gets from `rect`, with the bar
/// list `bars`.
fn top_level_bounds(fake: &Fake, bars: &[&str], rect: Rect) -> Option<(i32, i32, i32, i32)> {
    let mut state = fake.state(PopupEmbed::Composite);
    state.bars.extend(bars.iter().map(|name| fake.bar(name, 2)));
    resolve_anchor(&state, &popup_kind(), None, Some(rect), None).map(|anchor| {
        assert_eq!(anchor.bar_scale, 2);
        assert_eq!(anchor.target_monitor_id, MonitorId::new("DP-1"));
        (
            anchor.anchor_x,
            anchor.anchor_y,
            anchor.anchor_w,
            anchor.anchor_h,
        )
    })
}

#[test]
fn top_level_popup_anchors_to_the_whole_bar_column() {
    let fake = Fake::new();
    let rect = Rect::new(Position::new(2, 3), Size::new(5, 6));
    // The fixture's bar is 1 pixel high.
    assert_eq!(
        top_level_bounds(&fake, &["HDMI-A-1", "DP-1"], rect),
        Some((2, 0, 5, 1))
    );
}

#[test]
fn popup_on_a_monitor_without_a_bar_has_no_anchor() {
    let fake = Fake::new();
    let rect = Rect::new(Position::new(2, 3), Size::new(5, 6));
    assert_eq!(top_level_bounds(&fake, &["HDMI-A-1"], rect), None);
}
