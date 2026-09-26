use super::state::WaylandState;
use crate::features::layout_engine::domain::popup::descendants_deepest_first;
use crate::features::layout_engine::domain::FloatingKind;
use crate::shared::events::core::{InteractionEvent, SurfaceLifecycleEvent};

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
        let Some(floating) = state.floating_surfaces.remove(k) else {
            continue;
        };
        state.surface_to_id.remove(&floating.surface);
        for child in floating.module_surfaces.values() {
            state.surface_to_id.remove(&child.surface);
        }
        if i != last || notify_self {
            notify_dismissed(state, k);
        }
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
