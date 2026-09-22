use crate::features::layout_engine::domain::{
    DisplayCommand, DisplayCommandSender, FloatingKind, PopupTarget, RenderNode,
};
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::module_runtime::application::ModuleContext;
use crate::features::module_runtime::domain::{PointerAction, RenderOutcome};
use crate::features::module_runtime::ports::{AnyModulePort, LayoutEvent, LayoutEventSender};
use crate::features::vdom::domain::UiCommandSender;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::geometry::{Position, Scale};
use crate::shared::primitives::{ModuleId, MonitorId};
use crate::shared::rendering::ports::canvas::CanvasFactory;
use crate::shared::wayland::ports::DynSurfaceManager;
use std::collections::{HashMap, HashSet};

/// The set of monitors to render for. Wayland is the only subsystem that
/// knows whether a surface can exist at all, and it's authoritative here:
/// `monitor_scales` is populated the moment an output's `wl_output::Name`/
/// `Scale` event arrives and cleaned the moment its global is removed (see
/// `dispatch_output.rs` / `dispatch_registry.rs`). Hyprland's own view can
/// lag a hotplug by a beat (workspaces move onto a new monitor before
/// `monitoraddedv2` arrives), and `module_sizes` is a cache that only ever
/// gains entries — treating either as a discovery source let a disconnected
/// monitor stay "discovered" forever.
#[must_use]
pub(super) fn discover_monitors(hub: &SignalHub) -> Vec<MonitorId> {
    hub.monitor_scales_rx().borrow().keys().cloned().collect()
}

pub(super) fn dispatch_outcome_layouts<LS: LayoutEventSender>(
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

pub(super) fn dispatch_outcome_buffer(
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

pub(super) fn dispatch_outcome_popups<DS: DisplayCommandSender>(
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
            offset: anchored_popup.popup().offset(),
        });
    } else if active_popups.remove(monitor_id) {
        display_sender.send_display_command(DisplayCommand::HideFloatingSurface {
            kind: FloatingKind::Popup(PopupTarget::new(module_id, monitor_id.clone())),
        });
    }
}

pub(super) fn dispatch_outcome_panels<DS: DisplayCommandSender>(
    display_sender: &DS,
    active_panels: &mut HashSet<MonitorId>,
    module_id: ModuleId,
    monitor_id: &MonitorId,
    outcome: &RenderOutcome,
) {
    if let Some(active_panel) = outcome.render_tree().find_panel() {
        active_panels.insert(monitor_id.clone());
        display_sender.send_display_command(DisplayCommand::ShowFloatingSurface {
            kind: FloatingKind::Panel(PopupTarget::new(module_id, monitor_id.clone())),
            monitor_id: Some(monitor_id.clone()),
            anchor_rect: None,
            layout: Box::new(active_panel.layout().clone()),
            offset: None,
        });
    } else if active_panels.remove(monitor_id) {
        display_sender.send_display_command(DisplayCommand::HideFloatingSurface {
            kind: FloatingKind::Panel(PopupTarget::new(module_id, monitor_id.clone())),
        });
    }
}

pub(super) fn update_floating_trees<F: CanvasFactory>(
    canvas_factory: &mut F,
    popup_trees: &mut HashMap<MonitorId, RenderNode>,
    panel_trees: &mut HashMap<MonitorId, RenderNode>,
    monitor_id: &MonitorId,
    outcome: &RenderOutcome,
) {
    if let Some(anchored) = outcome.render_tree().find_popup_with_anchor() {
        let mut engine = crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter::new();
        let mut measurer = canvas_factory.create_text_measurer(
            Scale::new(1.0),
            crate::shared::config::domain::FontFamily::new(String::new()),
            crate::shared::config::domain::FontSize::new(14.0),
        );
        if let Ok(tree) = engine.calculate_layout(
            anchored.layout().clone(),
            &mut measurer,
            Position::new(0, 0),
        ) {
            popup_trees.insert(monitor_id.clone(), tree);
        }
    } else {
        popup_trees.remove(monitor_id);
    }

    if let Some(panel) = outcome.render_tree().find_panel() {
        let mut engine = crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter::new();
        let mut measurer = canvas_factory.create_text_measurer(
            Scale::new(1.0),
            crate::shared::config::domain::FontFamily::new(String::new()),
            crate::shared::config::domain::FontSize::new(14.0),
        );
        if let Ok(tree) =
            engine.calculate_layout(panel.layout().clone(), &mut measurer, Position::new(0, 0))
        {
            panel_trees.insert(monitor_id.clone(), tree);
        }
    } else {
        panel_trees.remove(monitor_id);
    }
}

pub(super) fn dispatch_post_actions<
    LS: LayoutEventSender,
    DS: DisplayCommandSender,
    US: UiCommandSender,
>(
    port: &mut Box<dyn AnyModulePort>,
    ctx: &ModuleContext<LS, DS, US>,
    actions: Vec<PointerAction>,
) {
    for action in actions {
        match action {
            PointerAction::CallFunction(func_name, mon_id) => {
                let _ = if let Some(m) = mon_id {
                    port.call_function_with_args(&func_name, &[m.as_str()])
                } else {
                    port.call_function(&func_name)
                };
            }
            PointerAction::SendUi(cmd) => {
                ctx.ui_sender().send_ui_command(cmd);
            }
            PointerAction::SendDisplay(cmd) => {
                ctx.display_sender().send_display_command(cmd);
            }
        }
    }
}
