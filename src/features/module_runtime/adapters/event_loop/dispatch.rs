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
/// `seen` is every monitor this actor has ever produced a render tree for
/// (`RenderPipeline::render_trees` keys) — a monitor already in `seen` but
/// absent from `layouts` has no live site any more (its embedding rect was
/// withdrawn) and must stay suspended (decision 14), not be rediscovered via
/// the Hyprland/module-sizes bootstrap path below. A monitor never rendered
/// before still needs that bootstrap path: it renders once, unconstrained,
/// purely to report an intrinsic size before anyone has assigned it a rect.
#[must_use]
pub(super) fn discover_monitors<V>(
    hub: &SignalHub,
    layouts: &HashMap<MonitorId, V>,
    seen: &HashSet<MonitorId>,
) -> Vec<MonitorId> {
    let mut all_monitors: HashSet<MonitorId> = HashSet::new();
    for m in layouts.keys() {
        all_monitors.insert(m.clone());
    }
    for m in hub.hyprland_rx().borrow().monitors().values() {
        let id = MonitorId::new(m.name().as_str());
        if !seen.contains(&id) {
            all_monitors.insert(id);
        }
    }
    for m in hub.module_sizes_rx().borrow().keys() {
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
