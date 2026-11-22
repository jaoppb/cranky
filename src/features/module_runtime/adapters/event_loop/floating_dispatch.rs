use crate::features::layout_engine::domain::{
    DisplayCommand, DisplayCommandSender, FloatingKind, PopupTarget, RenderNode, ShowFloating,
};
use crate::features::module_runtime::domain::render_pipeline::{
    FloatingHold, ScaledCanvasFactory, paint_pipeline,
};
use crate::features::module_runtime::domain::{PopupRenderLayout, RenderOutcome};
use crate::shared::primitives::geometry::{Rect, Scale};
use crate::shared::primitives::{ModuleId, MonitorId, PopupOffset};
use crate::shared::rendering::ports::canvas::CanvasFactory;
use std::collections::HashMap;

/// One module's cache of what it last showed for one floating kind, per monitor.
type FloatingTrees = HashMap<MonitorId, (Option<Rect>, RenderNode)>;

/// What one render asks of one floating kind.
struct FloatingRequest<'a> {
    kind: FloatingKind,
    node: Option<&'a RenderNode>,
    anchor_rect: Option<Rect>,
    offset: Option<PopupOffset>,
    /// Whether an embedded child has not settled on its host width yet.
    hold: FloatingHold,
}

/// Everything the three floating kinds of one module share on one monitor for
/// one render.
pub(super) struct FloatingDispatcher<'a, DS: DisplayCommandSender, F: CanvasFactory> {
    pub(super) display_sender: &'a DS,
    /// The module whose popup and panel these are, which names their surfaces.
    pub(super) module_id: ModuleId,
    pub(super) canvas_factory: &'a mut F,
    pub(super) scale: Scale,
    pub(super) monitor_id: &'a MonitorId,
    /// The floating surface this module is itself embedded in, if any: a
    /// popup, panel or tooltip opened from inside it nests under that one.
    pub(super) parent: Option<FloatingKind>,
}

impl<DS: DisplayCommandSender, F: CanvasFactory> FloatingDispatcher<'_, DS, F> {
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
    fn dispatch(&mut self, trees: &mut FloatingTrees, request: FloatingRequest<'_>) {
        // An embedded child has not settled on its host width yet: leave the
        // surface exactly as it is (closed, or showing its previous frame)
        // rather than present a frame at the wrong width.
        if request.hold == FloatingHold::Held {
            return;
        }
        let Some(node) = request.node else {
            self.hide(trees, request.kind);
            return;
        };

        let anchor_rect = request.anchor_rect;
        let unchanged = trees
            .get(self.monitor_id)
            .is_some_and(|(cached_anchor, cached_node)| {
                *cached_anchor == anchor_rect && cached_node == node
            });
        trees.insert(self.monitor_id.clone(), (anchor_rect, node.clone()));
        if unchanged {
            return;
        }
        self.show(node, request);
    }

    /// Forgets what this monitor showed for `kind`, and hides its surface if anything was shown.
    fn hide(&self, trees: &mut FloatingTrees, kind: FloatingKind) {
        if trees.remove(self.monitor_id).is_some() {
            self.display_sender
                .send_display_command(DisplayCommand::HideFloatingSurface { kind });
        }
    }

    /// Paints `node`, the request's content, at this monitor's scale and shows it at the
    /// request's anchor and offset, nested under `self.parent`; nothing when it cannot be painted.
    fn show(&mut self, node: &RenderNode, request: FloatingRequest<'_>) {
        let logical_size = *node.rect().size();
        let Some((buffer, _)) = paint_pipeline(
            node,
            Some(node.rect()),
            ScaledCanvasFactory::new(self.canvas_factory, self.scale),
        ) else {
            return;
        };
        self.display_sender
            .send_display_command(DisplayCommand::ShowFloatingSurface(ShowFloating {
                kind: request.kind,
                monitor_id: Some(self.monitor_id.clone()),
                anchor_rect: request.anchor_rect,
                buffer,
                logical_size,
                offset: request.offset,
                parent: self.parent.clone(),
            }));
    }

    pub(super) fn popups(&mut self, trees: &mut FloatingTrees, outcome: &RenderOutcome) {
        let popup_layout = outcome.popup_layout();
        let request = FloatingRequest {
            kind: FloatingKind::Popup(PopupTarget::new(self.module_id, self.monitor_id.clone())),
            node: popup_layout.map(PopupRenderLayout::node),
            anchor_rect: popup_layout.map(PopupRenderLayout::anchor_rect),
            offset: popup_layout.and_then(PopupRenderLayout::offset),
            hold: outcome.popup_hold(),
        };
        self.dispatch(trees, request);
    }

    pub(super) fn panels(&mut self, trees: &mut FloatingTrees, outcome: &RenderOutcome) {
        let request = FloatingRequest {
            kind: FloatingKind::Panel(PopupTarget::new(self.module_id, self.monitor_id.clone())),
            node: outcome.panel_layout(),
            anchor_rect: None,
            offset: None,
            hold: outcome.panel_hold(),
        };
        self.dispatch(trees, request);
    }

    /// A tooltip triggered on an element inside a module embedded in an open
    /// popup/panel (`ui.module()` nested inside floating content) nests the
    /// same way that module's own popup/panel would: under `self.parent`, not
    /// always under nothing.
    pub(super) fn tooltips(&mut self, trees: &mut FloatingTrees, outcome: &RenderOutcome) {
        let request = FloatingRequest {
            kind: FloatingKind::Tooltip,
            node: outcome.tooltip_layout(),
            anchor_rect: None,
            offset: None,
            hold: outcome.tooltip_hold(),
        };
        self.dispatch(trees, request);
    }
}
