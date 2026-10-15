use crate::features::layout_engine::domain::{
    DisplayCommand, DisplayCommandSender, FloatingKind, PopupTarget, RenderNode, ShowFloating,
};
use crate::features::module_runtime::domain::render_pipeline::paint_pipeline;
use crate::features::module_runtime::domain::{PopupRenderLayout, RenderOutcome};
use crate::shared::primitives::geometry::{Rect, Scale};
use crate::shared::primitives::{ModuleId, MonitorId, PopupOffset};
use crate::shared::rendering::ports::canvas::CanvasFactory;
use std::collections::HashMap;

/// Shows, updates or hides one floating kind for one monitor.
///
/// `trees` is this module's own cache of what it last showed for `kind` on
/// this monitor — the same cache the module's hit-testing reads, so there is
/// exactly one place that remembers "what's currently there," not a
/// hit-testing copy and a separate dedup copy. The cached pair is the
/// content *and* the anchor it was positioned against: content identical to
/// what's cached is never repainted or resent, but a dispatch still goes out
/// whenever the anchor moved even if the content didn't, since that's what
/// actually repositions the floating surface on screen (a sibling module's
/// width changing this module's x-offset on the same render tick, while the
/// popup's own content stays the same). Comparing the node alone would miss
/// that and leave the popup visually stuck at its old position.
#[allow(clippy::too_many_arguments)]
fn dispatch_floating_kind<DS: DisplayCommandSender, F: CanvasFactory>(
    display_sender: &DS,
    trees: &mut HashMap<MonitorId, (Option<Rect>, RenderNode)>,
    canvas_factory: &mut F,
    scale: Scale,
    kind: FloatingKind,
    monitor_id: &MonitorId,
    node: Option<&RenderNode>,
    anchor_rect: Option<Rect>,
    offset: Option<PopupOffset>,
    parent: Option<FloatingKind>,
) {
    let Some(node) = node else {
        if trees.remove(monitor_id).is_some() {
            display_sender.send_display_command(DisplayCommand::HideFloatingSurface { kind });
        }
        return;
    };

    let unchanged = trees
        .get(monitor_id)
        .is_some_and(|(cached_anchor, cached_node)| *cached_anchor == anchor_rect && cached_node == node);
    trees.insert(monitor_id.clone(), (anchor_rect, node.clone()));
    if unchanged {
        return;
    }

    let logical_size = *node.rect().size();
    let Some((buffer, _)) = paint_pipeline(node, Some(node.rect()), scale, canvas_factory) else {
        return;
    };
    display_sender.send_display_command(DisplayCommand::ShowFloatingSurface(ShowFloating {
        kind,
        monitor_id: Some(monitor_id.clone()),
        anchor_rect,
        buffer,
        logical_size,
        offset,
        parent,
    }));
}

#[allow(clippy::too_many_arguments)]
pub(super) fn dispatch_outcome_popups<DS: DisplayCommandSender, F: CanvasFactory>(
    display_sender: &DS,
    popup_render_trees: &mut HashMap<MonitorId, (Option<Rect>, RenderNode)>,
    canvas_factory: &mut F,
    scale: Scale,
    module_id: ModuleId,
    monitor_id: &MonitorId,
    outcome: &RenderOutcome,
    parent: Option<FloatingKind>,
) {
    let kind = FloatingKind::Popup(PopupTarget::new(module_id, monitor_id.clone()));
    let popup_layout = outcome.popup_layout();
    dispatch_floating_kind(
        display_sender,
        popup_render_trees,
        canvas_factory,
        scale,
        kind,
        monitor_id,
        popup_layout.map(PopupRenderLayout::node),
        popup_layout.map(PopupRenderLayout::anchor_rect),
        popup_layout.and_then(PopupRenderLayout::offset),
        parent,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn dispatch_outcome_panels<DS: DisplayCommandSender, F: CanvasFactory>(
    display_sender: &DS,
    panel_render_trees: &mut HashMap<MonitorId, (Option<Rect>, RenderNode)>,
    canvas_factory: &mut F,
    scale: Scale,
    module_id: ModuleId,
    monitor_id: &MonitorId,
    outcome: &RenderOutcome,
    parent: Option<FloatingKind>,
) {
    let kind = FloatingKind::Panel(PopupTarget::new(module_id, monitor_id.clone()));
    dispatch_floating_kind(
        display_sender,
        panel_render_trees,
        canvas_factory,
        scale,
        kind,
        monitor_id,
        outcome.panel_layout(),
        None,
        None,
        parent,
    );
}

pub(super) fn dispatch_outcome_tooltips<DS: DisplayCommandSender, F: CanvasFactory>(
    display_sender: &DS,
    tooltip_render_trees: &mut HashMap<MonitorId, (Option<Rect>, RenderNode)>,
    canvas_factory: &mut F,
    scale: Scale,
    monitor_id: &MonitorId,
    outcome: &RenderOutcome,
    parent: Option<FloatingKind>,
) {
    // A tooltip triggered on an element inside a module embedded in an
    // open popup/panel (`ui.module()` nested inside floating content) nests
    // the same way that module's own popup/panel would — `parent` is the
    // same floating surface `dispatch_outcome_popups`/`dispatch_outcome_panels`
    // compute for this module, not always `None`.
    dispatch_floating_kind(
        display_sender,
        tooltip_render_trees,
        canvas_factory,
        scale,
        FloatingKind::Tooltip,
        monitor_id,
        outcome.tooltip_layout(),
        None,
        None,
        parent,
    );
}
