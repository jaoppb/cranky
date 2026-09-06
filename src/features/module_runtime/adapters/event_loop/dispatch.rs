use crate::features::layout_engine::domain::{DisplayCommand, FloatingKind, PopupTarget};
use crate::features::module_runtime::domain::RenderOutcome;
use crate::features::module_runtime::ports::LayoutEvent;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::geometry::Rect;
use crate::shared::primitives::{ModuleId, MonitorId};
use crate::shared::wayland::ports::DynSurfaceManager;
use std::collections::{HashMap, HashSet};

#[allow(clippy::implicit_hasher)]
#[must_use]
pub fn discover_monitors(hub: &SignalHub, layouts: &HashMap<MonitorId, Rect>) -> Vec<MonitorId> {
    let mut all_monitors: HashSet<MonitorId> = HashSet::new();
    for m in hub.hyprland_rx().borrow().monitors().values() {
        all_monitors.insert(MonitorId::new(m.name().as_str()));
    }
    for m in layouts.keys() {
        all_monitors.insert(m.clone());
    }
    for m in hub.module_sizes_rx().borrow().keys() {
        all_monitors.insert(m.clone());
    }
    all_monitors.into_iter().collect()
}

pub fn dispatch_outcome_layouts<LS: crate::features::module_runtime::ports::LayoutEventSender>(
    layout_sender: &LS,
    module_id: ModuleId,
    monitor_id: &MonitorId,
    outcome: &RenderOutcome,
) {
    if !outcome.child_layouts().is_empty() {
        layout_sender.send_layout_event(LayoutEvent::ContainerLayoutsCalculated {
            parent_id: module_id,
            monitor_id: monitor_id.clone(),
            layouts: outcome.child_layouts().to_vec(),
        });
    }

    if let Some(size_change) = outcome.size_change() {
        layout_sender.send_layout_event(LayoutEvent::ModuleSizeChanged {
            monitor_id: monitor_id.clone(),
            module_id,
            size: size_change.new_size(),
        });
    }
}

pub fn dispatch_outcome_buffer(
    surface_manager: &DynSurfaceManager,
    module_id: ModuleId,
    parent_id: Option<ModuleId>,
    monitor_id: &MonitorId,
    outcome: &RenderOutcome,
) {
    if let Some((buffer, position)) = outcome.buffer().cloned() {
        surface_manager.submit_child_buffer(
            module_id,
            parent_id,
            monitor_id.clone(),
            position,
            buffer,
        );
    }
}

#[allow(clippy::implicit_hasher)]
pub fn dispatch_outcome_popups<DS: crate::features::layout_engine::domain::DisplayCommandSender>(
    display_sender: &DS,
    active_popups: &mut HashSet<MonitorId>,
    module_id: ModuleId,
    monitor_id: &MonitorId,
    outcome: &RenderOutcome,
) {
    if let Some(anchored_popup) = outcome.render_tree().find_popup_with_anchor() {
        active_popups.insert(monitor_id.clone());
        display_sender.send_display_command(DisplayCommand::ShowFloatingSurface {
            kind: FloatingKind::Popup(PopupTarget::new(module_id, monitor_id.clone())),
            monitor_id: Some(monitor_id.clone()),
            anchor_rect: Some(*anchored_popup.anchor_rect()),
            layout: Box::new(anchored_popup.layout().clone()),
        });
    } else if active_popups.remove(monitor_id) {
        display_sender.send_display_command(DisplayCommand::HideFloatingSurface {
            kind: FloatingKind::Popup(PopupTarget::new(module_id, monitor_id.clone())),
        });
    }
}
