use super::dispatch_pointer::send;
use super::state::WaylandState;
use super::types::FloatingSurface;
use crate::features::layout_engine::domain::popup::descendants_deepest_first;
use crate::features::layout_engine::domain::FloatingKind;
use crate::shared::events::core::{InteractionEvent, PointerEvent, SurfaceLifecycleEvent};

/// Tears down `kind` and every floating surface nested inside it (decision
/// 7), deepest first — destroying a non-topmost `xdg_popup` is a protocol
/// error, so a parent can never be removed before its own children are.
///
/// `notify_self` controls whether `kind` itself gets a
/// `PopupDismissed`/`PanelDismissed` sent back to its owning module: `false`
/// when that module closed its own popup (it already knows), `true` for
/// every other teardown path — an exclusivity conflict or a
/// compositor-driven `PopupDone`. Every descendant is always notified,
/// regardless of `notify_self`: its owner needs to clear its own
/// `popup_render_trees`/`panel_render_trees` cache and run its dismiss
/// handler, or a suspended child could resurrect an in-place popup, or
/// dedup could swallow a reopen, the next time its parent's popup opens
/// again.
pub(crate) fn teardown_floating(state: &mut WaylandState, kind: &FloatingKind, notify_self: bool) {
    let links: Vec<(FloatingKind, Option<FloatingKind>)> = state
        .floating_surfaces
        .iter()
        .map(|(k, f)| (k.clone(), f.parent.clone()))
        .collect();
    let mut order = descendants_deepest_first(kind, &links);
    order.push(kind.clone());

    let Some(last) = order.len().checked_sub(1) else {
        return;
    };
    for (i, k) in order.iter().enumerate() {
        state.pending_children.remove(k);
        let Some(floating) = state.floating_surfaces.remove(k) else {
            continue;
        };
        leave_dying_surface(state, &floating);
        state.surface_to_id.remove(&floating.surface);
        for child in floating.module_surfaces.values() {
            state.surface_to_id.remove(&child.surface);
        }
        if i != last || notify_self {
            notify_dismissed(state, k);
        }
    }
}

/// If the pointer is on `floating` — its owner's own area, a child
/// composited into it, or an old-style child subsurface — whatever it
/// targets lives on a surface about to be destroyed: that target gets its
/// `PointerLeave` now and any grab frozen onto it ends.
///
/// Keyed on `pointer_surface` rather than on the target's identity because
/// `pointer_target` is always routed from `pointer_surface` (only Enter and
/// Leave change the surface, and both end a grab), so this is exact even
/// if the same module also has a target elsewhere. A compositor Leave
/// arriving after this finds no target and sends nothing; one arriving
/// before already took it, leaving nothing here to send.
fn leave_dying_surface(state: &mut WaylandState, floating: &FloatingSurface) {
    let Some(surface) = state.pointer_surface.as_ref() else {
        return;
    };
    let on_floating = *surface == floating.surface
        || floating
            .module_surfaces
            .values()
            .any(|m| m.surface == *surface);
    if !on_floating {
        return;
    }
    state.pointer_grab.cancel();
    if let Some(target) = state.pointer_target.take() {
        let surface = target.2;
        send(state, &target, PointerEvent::PointerLeave { surface });
    }
}

fn notify_dismissed(state: &WaylandState, kind: &FloatingKind) {
    let (target, event) = match kind {
        FloatingKind::Popup(t) => (t, SurfaceLifecycleEvent::PopupDismissed),
        FloatingKind::Panel(t) => (t, SurfaceLifecycleEvent::PanelDismissed),
        FloatingKind::Tooltip => return,
    };
    let _ = state.hub.pointer_tx().send((
        target.module_id(),
        target.monitor_id().clone(),
        InteractionEvent::Lifecycle(event),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::layout_engine::domain::PopupTarget;
    use crate::shared::config::domain::PopupEmbed;
    use crate::shared::events::core::SurfaceKind;
    use crate::shared::primitives::{ModuleId, MonitorId, Position, Size};
    use crate::shared::wayland::adapters::wayland::dispatch_pointer::{
        handle_pointer_enter, handle_pointer_leave, handle_pointer_motion,
    };
    use crate::shared::wayland::adapters::wayland::dispatch_pointer_button::handle_pointer_button;
    use crate::shared::wayland::adapters::wayland::floating_surface::hide_floating;
    use crate::shared::wayland::adapters::wayland::tests::fixture::pointer::{
        CHILD, Event, LEFT, OWNER, PRESSED, bar, bar_module, child, drain, enter, leave, owner,
        popup,
    };
    use crate::shared::wayland::adapters::wayland::tests::fixture::{Fake, popup_kind, solid};
    use crate::shared::wayland::composite::CompositeLayers;

    fn dismissed() -> Event {
        let event = InteractionEvent::Lifecycle(SurfaceLifecycleEvent::PopupDismissed);
        (ModuleId::new(OWNER), MonitorId::new("DP-1"), event)
    }

    fn pending_layer() -> CompositeLayers {
        let mut layers = CompositeLayers::default();
        let (id, monitor) = (ModuleId::new(CHILD), MonitorId::new("DP-1"));
        layers.upsert(id, monitor, Position::new(0, 0), solid(1, 1, [1; 4]));
        layers
    }

    /// Teardown first, the compositor's Leave for the dead surface after.
    #[test]
    fn teardown_clears_pointer_target_and_sends_leave() {
        let fake = Fake::new();
        let (mut state, surface) = popup(&fake, 1);
        let mut rx = state.hub.pointer_rx();
        handle_pointer_enter(&mut state, &surface, 11.0, 3.0);
        drain(&mut rx);
        teardown_floating(&mut state, &popup_kind(), true);
        let expected = vec![child(leave(SurfaceKind::Bar)), dismissed()];
        assert_eq!(drain(&mut rx), expected);
        assert_eq!(state.pointer_target, None);
        handle_pointer_leave(&mut state);
        assert_eq!(drain(&mut rx), vec![]);
    }

    /// The compositor's Leave first, teardown after.
    #[test]
    fn teardown_after_compositor_leave_sends_no_second_leave() {
        let fake = Fake::new();
        let (mut state, surface) = popup(&fake, 1);
        let mut rx = state.hub.pointer_rx();
        handle_pointer_enter(&mut state, &surface, 11.0, 3.0);
        drain(&mut rx);
        handle_pointer_leave(&mut state);
        teardown_floating(&mut state, &popup_kind(), true);
        let expected = vec![child(leave(SurfaceKind::Bar)), dismissed()];
        assert_eq!(drain(&mut rx), expected);
    }

    /// Hyprland enters the surface underneath without a Leave for the dead
    /// one; the child already got its Leave, so it gets no second one.
    #[test]
    fn teardown_then_enter_elsewhere_sends_no_stale_leave() {
        let fake = Fake::new();
        let (mut state, surface) = popup(&fake, 1);
        let bar_surface = bar_module(&fake, &mut state);
        let mut rx = state.hub.pointer_rx();
        handle_pointer_enter(&mut state, &surface, 11.0, 3.0);
        drain(&mut rx);
        hide_floating(&mut state, &popup_kind());
        assert_eq!(drain(&mut rx), vec![child(leave(SurfaceKind::Bar))]);
        handle_pointer_enter(&mut state, &bar_surface, 4.0, 4.0);
        assert_eq!(drain(&mut rx), vec![bar(enter(SurfaceKind::Bar))]);
    }

    #[test]
    fn teardown_cancels_grab_frozen_on_torn_down_child() {
        let fake = Fake::new();
        let (mut state, surface) = popup(&fake, 1);
        let mut rx = state.hub.pointer_rx();
        handle_pointer_enter(&mut state, &surface, 11.0, 3.0);
        handle_pointer_button(&mut state, LEFT, PRESSED, 7);
        drain(&mut rx);
        hide_floating(&mut state, &popup_kind());
        assert_eq!(drain(&mut rx), vec![child(leave(SurfaceKind::Bar))]);
        assert!(!state.pointer_grab.is_active());
        handle_pointer_motion(&mut state, 12.0, 3.0);
        assert_eq!(drain(&mut rx), vec![]);
    }

    #[test]
    fn teardown_while_hovering_owner_area_leaves_owner() {
        let fake = Fake::new();
        let (mut state, surface) = popup(&fake, 1);
        let mut rx = state.hub.pointer_rx();
        handle_pointer_enter(&mut state, &surface, 1.0, 1.0);
        drain(&mut rx);
        hide_floating(&mut state, &popup_kind());
        assert_eq!(drain(&mut rx), vec![owner(leave(SurfaceKind::Popup))]);
        assert_eq!(state.pointer_target, None);
    }

    #[test]
    fn teardown_keeps_pointer_target_on_other_surface() {
        let fake = Fake::new();
        let (mut state, _) = popup(&fake, 1);
        let bar_surface = bar_module(&fake, &mut state);
        handle_pointer_enter(&mut state, &bar_surface, 4.0, 4.0);
        let before = state.pointer_target.clone();
        let mut rx = state.hub.pointer_rx();
        hide_floating(&mut state, &popup_kind());
        assert_eq!(drain(&mut rx), vec![]);
        assert!(before.is_some());
        assert_eq!(state.pointer_target, before);
    }

    #[test]
    fn teardown_and_hide_discard_pending_children() {
        let fake = Fake::new();
        // Hidden before it was ever shown: only its pending children exist.
        let mut state = fake.state(PopupEmbed::Composite);
        state.pending_children.insert(popup_kind(), pending_layer());
        hide_floating(&mut state, &popup_kind());
        assert!(state.pending_children.is_empty());

        // Torn down with a nested popup: both kinds' pending children go.
        let (mut state, _) = popup(&fake, 1);
        let nested = FloatingKind::Popup(PopupTarget::new(
            ModuleId::new(CHILD),
            MonitorId::new("DP-1"),
        ));
        let mut floating = fake.floating(Size::new(1, 1), solid(1, 1, [0; 4]));
        floating.parent = Some(popup_kind());
        state.floating_surfaces.insert(nested.clone(), floating);
        state.pending_children.insert(popup_kind(), pending_layer());
        state.pending_children.insert(nested, pending_layer());
        teardown_floating(&mut state, &popup_kind(), true);
        assert!(state.pending_children.is_empty());
    }
}
