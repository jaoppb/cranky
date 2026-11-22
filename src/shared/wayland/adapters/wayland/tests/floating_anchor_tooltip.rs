use crate::features::layout_engine::domain::{FloatingKind, PopupTarget, ShowFloating};
use crate::shared::config::domain::PopupEmbed;
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::primitives::{ModuleId, MonitorId};
use crate::shared::wayland::adapters::wayland::dispatch_pointer::handle_pointer_enter;
use crate::shared::wayland::adapters::wayland::floating_anchor::{
    FloatingParentRole, resolve_anchor,
};
use crate::shared::wayland::adapters::wayland::tests::fixture::pointer::popup;
use crate::shared::wayland::adapters::wayland::tests::fixture::{
    Fake, popup_kind, show_request, solid,
};

/// Regression test (Gap A): the hovered element can belong to a module
/// that isn't embedded in the bar at all, but in a module embedded inside
/// an already-open popup (`ui.module()` nested inside floating content).
/// `resolve_anchor` must find it by also scanning
/// `WaylandState::floating_surfaces` for a `module_surfaces` match, the
/// same way the `Popup`/`Panel` arms already scan a bar's — not just
/// return `None` because no bar owns the hovered surface.
#[test]
fn test_resolve_anchor_tooltip_finds_module_embedded_in_open_popup() {
    let fake = Fake::new();
    let mut state = fake.state(PopupEmbed::Composite);
    // The already-open popup's own surface/xdg_surface — what the embedded
    // module's tooltip must nest inside.
    let empty = RenderBuffer::new(Vec::new(), Size::new(0, 0));
    let mut popup = fake.floating(Size::new(100, 100), empty);
    // The module embedded inside that popup's own content (decision 7, gap
    // 4): its own subsurface, parented to the popup's surface rather than
    // to a bar's.
    let mut module = fake.module_surface(&popup.surface);
    (module.x, module.y) = (5, 3);
    // The pointer is over the embedded module's own subsurface, not any bar
    // surface.
    state.pointer_surface = Some(module.surface.clone());
    state.pointer_pos = (12.0, 4.0);
    popup.module_surfaces.insert(ModuleId::new(7), module);
    let owner = PopupTarget::new(ModuleId::new(1), MonitorId::new("DP-1"));
    state
        .floating_surfaces
        .insert(FloatingKind::Popup(owner), popup);

    let anchor = resolve_anchor(&state, &tooltip_request())
        .expect("tooltip anchor must resolve for a module embedded in an open popup");

    assert!(
        matches!(anchor.parent_role, FloatingParentRole::Popup(_)),
        "tooltip must nest inside the popup's xdg_surface, not fall back to a bar's layer surface"
    );
    assert_eq!(anchor.target_monitor_id, MonitorId::new("DP-1"));
    // pointer_pos.0 (12) + the embedded module's own subsurface offset
    // (5), exactly the transform the bar-owned case already applies.
    assert_eq!(anchor.anchor_x, 17);
}

#[test]
fn tooltip_anchor_resolves_for_composite_child() {
    let fake = Fake::new();
    let (mut state, surface) = popup(&fake, 1);
    handle_pointer_enter(&mut state, &surface, (12.0, 4.0));
    (state.bars[0].scale, state.bars[0].height) = (3, 24);
    let anchor = resolve_anchor(&state, &tooltip_request())
        .expect("tooltip anchor resolves for a composited child");
    let popup_xdg = state
        .floating_surfaces
        .get(&popup_kind())
        .map(|floating| floating.xdg_surface.clone());
    assert!(
        matches!(&anchor.parent_role, FloatingParentRole::Popup(x) if Some(x) == popup_xdg.as_ref())
    );
    assert_eq!(anchor.target_monitor_id, MonitorId::new("DP-1"));
    // Already popup-local: no layer origin added on top. A tooltip anchors
    // on one point below its bar, at the bar's scale.
    let placed = (
        anchor.anchor_x,
        anchor.anchor_y,
        anchor.anchor_w,
        anchor.anchor_h,
        anchor.bar_scale,
    );
    assert_eq!(placed, (12, 24, 1, 1, 3));
}

#[test]
fn tooltip_anchor_ignores_popup_owner_own_content() {
    let fake = Fake::new();
    let (mut state, surface) = popup(&fake, 1);
    handle_pointer_enter(&mut state, &surface, (1.0, 1.0));
    assert!(resolve_anchor(&state, &tooltip_request()).is_none());
}

fn tooltip_request() -> ShowFloating {
    show_request(FloatingKind::Tooltip, solid(1, 1, [0; 4]), None)
}
