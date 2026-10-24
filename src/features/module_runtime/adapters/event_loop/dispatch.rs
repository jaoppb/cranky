use crate::features::layout_engine::domain::{FloatingKind, PopupTarget};
use crate::features::module_runtime::domain::RenderOutcome;
use crate::features::module_runtime::ports::{LayoutEvent, LayoutEventSender};
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::{LayoutSurface, ModuleId, MonitorId};
use crate::shared::wayland::ports::{DynSurfaceManager, SurfaceParent};
use std::collections::{HashMap, HashSet};

/// Only ever consults which monitors have *some* entry, never a value — kept
/// generic so it isn't coupled to whatever a layout channel's value type is.
///
/// A top-level actor (`is_embedded` false — the bar itself, or any module
/// with no parent) has no "site" to lose: it always covers every monitor
/// Wayland currently reports, on every pass. Suspension (decision 14) only
/// makes sense for an *embedded* actor, whose presence on a monitor is a
/// fact some parent asserts by giving it a rect in `layouts` — for that
/// actor, `seen` (every monitor this actor has ever produced a render tree
/// for, i.e. `RenderPipeline::render_trees` keys) matters: a monitor already
/// in `seen` but absent from `layouts` has no live site any more (its
/// embedding rect was withdrawn) and must stay suspended, not be
/// rediscovered via the bootstrap path below. A monitor this embedded actor
/// never rendered before still needs that bootstrap path: it renders once,
/// unconstrained, purely to report an intrinsic size before its parent has
/// assigned it a rect.
///
/// Wayland's `monitor_scales` is the sole authority on whether a monitor
/// exists at all: it's populated the moment an output's `wl_output::Name`/
/// `Scale` event arrives and cleaned the moment its global is removed,
/// whereas Hyprland's own view can lag a hotplug by a beat (workspaces move
/// onto a new monitor before `monitoraddedv2` arrives) and `module_sizes` is
/// a cache that only ever gains entries — unioning either of those in, as
/// this used to, let a disconnected monitor stay "discovered" forever. A
/// `layouts` entry for a monitor Wayland doesn't (or no longer) know about
/// is likewise never a discovery source on its own — layout is a
/// consequence of a Wayland surface existing, not the other way round.
#[must_use]
pub(super) fn discover_monitors<V>(
    hub: &SignalHub,
    layouts: &HashMap<MonitorId, V>,
    seen: &HashSet<MonitorId>,
    is_embedded: bool,
) -> Vec<MonitorId> {
    let wayland_monitors: HashSet<MonitorId> =
        hub.monitor_scales_rx().borrow().keys().cloned().collect();
    if !is_embedded {
        return wayland_monitors.into_iter().collect();
    }
    let mut all_monitors: HashSet<MonitorId> = HashSet::new();
    for m in layouts.keys() {
        if wayland_monitors.contains(m) {
            all_monitors.insert(m.clone());
        }
    }
    for m in &wayland_monitors {
        if !seen.contains(m) {
            all_monitors.insert(m.clone());
        }
    }
    all_monitors.into_iter().collect()
}

pub(super) fn dispatch_outcome_layouts<LS: LayoutEventSender>(
    layout_sender: &LS,
    module_id: ModuleId,
    monitor_id: &MonitorId,
    outcome: &RenderOutcome,
    child_layouts_present: &mut HashMap<MonitorId, bool>,
) {
    let has_children_now = !outcome.child_layouts().is_empty();
    let had_children_before = child_layouts_present
        .get(monitor_id)
        .copied()
        .unwrap_or(false);

    // Sent whenever there's something to report, and once more — with an
    // empty list — the render where a parent's last child disappears (a
    // popup that just closed): `AppState` needs that transition to clear
    // the stale `computed_layouts` entry and suspend the child (decision
    // 14). A parent that never had children stays silent, exactly as
    // before.
    if has_children_now || had_children_before {
        layout_sender.send_layout_event(LayoutEvent::ContainerLayoutsCalculated {
            parent_id: module_id,
            monitor_id: monitor_id.clone(),
            layouts: outcome.child_layouts().to_vec(),
        });
    }
    child_layouts_present.insert(monitor_id.clone(), has_children_now);

    if let Some(size_change) = outcome.size_change() {
        layout_sender.send_layout_event(LayoutEvent::ModuleSizeChanged {
            monitor_id: monitor_id.clone(),
            module_id,
            size: size_change.new_size(),
            settled_width: size_change.settled_width(),
        });
    }
}

/// Builds this child's `SurfaceParent` from its fixed embedding `surface`
/// (decision 5: identity, and so embedding site, is fixed for the actor's
/// life) plus the monitor being rendered — a popup/panel's `FloatingKind`
/// needs both `parent_id` and `monitor_id` to name the exact floating
/// surface this child lives inside.
pub(super) fn surface_parent_for(
    parent_id: ModuleId,
    surface: LayoutSurface,
    monitor_id: &MonitorId,
) -> SurfaceParent {
    match surface {
        LayoutSurface::Bar => SurfaceParent::Bar,
        LayoutSurface::Popup => SurfaceParent::Floating(FloatingKind::Popup(PopupTarget::new(
            parent_id,
            monitor_id.clone(),
        ))),
        LayoutSurface::Panel => SurfaceParent::Floating(FloatingKind::Panel(PopupTarget::new(
            parent_id,
            monitor_id.clone(),
        ))),
        LayoutSurface::Tooltip => SurfaceParent::Floating(FloatingKind::Tooltip),
    }
}

pub(super) fn dispatch_outcome_buffer(
    surface_manager: &DynSurfaceManager,
    module_id: ModuleId,
    parent_id: Option<ModuleId>,
    surface: LayoutSurface,
    monitor_id: &MonitorId,
    outcome: &RenderOutcome,
) {
    if let Some((buffer, position)) = outcome.buffer().cloned() {
        let parent = parent_id.map(|pid| surface_parent_for(pid, surface, monitor_id));
        surface_manager.submit_child_buffer(
            module_id,
            parent,
            monitor_id.clone(),
            position,
            buffer,
        );
    }
}
