use super::runner::EventLoop;
use crate::features::layout_engine::domain::DisplayCommandSender;
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::module_runtime::domain::{LayoutContext, RenderOutcome};
use crate::features::module_runtime::ports::LayoutEventSender;
use crate::features::vdom::domain::InteractionContext;
use crate::features::vdom::domain::UiCommandSender;
use crate::shared::primitives::geometry::Scale;
use crate::shared::primitives::{
    ChildBounds, ModuleId, ModuleKey, MonitorFocus, MonitorId, SizeConstraint,
};
use crate::shared::rendering::ports::canvas::CanvasFactory;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

/// What every monitor of one render pass is rendered with: the bounds the parent gives the
/// module on each monitor, the failures of its own children, and each monitor's bar-tree layout
/// engine.
struct RenderPass<'a, E> {
    layouts: &'a HashMap<MonitorId, ChildBounds>,
    child_errors: &'a HashMap<ModuleKey, String>,
    layout_engines: &'a mut HashMap<MonitorId, E>,
}

impl<
    F: CanvasFactory + 'static,
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
> EventLoop<F, LS, DS, US>
{
    pub(crate) fn render_all_monitors<E: LayoutEnginePort + Default>(
        &mut self,
        layout_engines: &mut HashMap<MonitorId, E>,
    ) {
        let started = Instant::now();
        let layouts: HashMap<MonitorId, ChildBounds> = self.ctx.rxs_mut().0.borrow().clone();
        let monitors = self.discover_monitors(&layouts);
        self.forget_gone_monitors(layout_engines, &monitors);

        let child_errors = self.child_errors();
        let mut pass = RenderPass {
            layouts: &layouts,
            child_errors: &child_errors,
            layout_engines,
        };
        for monitor_id in monitors {
            self.render_and_dispatch(&mut pass, &monitor_id);
        }

        log_ui_updated(self.ctx.id(), started);
    }

    /// Drops the layout engine, pipeline state, floating trees, child-layout flag and pointer
    /// state of every monitor not in `monitors`.
    ///
    /// `layout_engines` and `self.render_pipeline` are both keyed by
    /// monitor and only ever grow (`.entry().or_insert_with()` /
    /// `.insert()`) — nothing else removes an entry when a monitor
    /// disconnects. `monitors` is the current, authoritative set for this
    /// pass, so anything outside it is for a monitor that's gone.
    fn forget_gone_monitors<E>(
        &mut self,
        layout_engines: &mut HashMap<MonitorId, E>,
        monitors: &[MonitorId],
    ) {
        let live: HashSet<MonitorId> = monitors.iter().cloned().collect();
        layout_engines.retain(|id, _| live.contains(id));
        self.render_pipeline.retain_monitors(&live);
        self.popup_render_trees.retain(|id, _| live.contains(id));
        self.panel_render_trees.retain(|id, _| live.contains(id));
        self.tooltip_render_trees.retain(|id, _| live.contains(id));
        self.child_layouts_present.retain(|id, _| live.contains(id));
        self.pointer_handler.retain_monitors(&live);
    }

    /// The reasons this module's own children failed to spawn, by child.
    fn child_errors(&self) -> HashMap<ModuleKey, String> {
        let self_id = self.ctx.id();
        self.ctx
            .hub()
            .module_errors_rx()
            .borrow()
            .iter()
            .filter(|(site, _)| site.parent() == Some(self_id))
            .map(|(site, reason)| (site.key().clone(), reason.clone()))
            .collect()
    }

    /// Renders `monitor_id` and dispatches the outcome, if the render produced one.
    fn render_and_dispatch<E: LayoutEnginePort + Default>(
        &mut self,
        pass: &mut RenderPass<'_, E>,
        monitor_id: &MonitorId,
    ) {
        let Some(outcome) = self.render_monitor(pass, monitor_id) else {
            return;
        };
        self.dispatch_render_outcome(monitor_id, &outcome);
    }

    /// Runs the diff, layout and paint phases on `monitor_id`, with what the hub and the
    /// pointer state say about that monitor now.
    fn render_monitor<E: LayoutEnginePort + Default>(
        &mut self,
        pass: &mut RenderPass<'_, E>,
        monitor_id: &MonitorId,
    ) -> Option<RenderOutcome> {
        let current = pass.layouts.get(monitor_id).copied();
        let module_sizes_guard = self.ctx.hub().module_sizes_rx().borrow().clone();
        let interaction_context = self.interaction_on(monitor_id);
        // Fresh per render, never cached: popup/panel content doesn't
        // need incremental diffing the way the bar tree does, and two
        // separate instances keep an open popup and an open panel from
        // diffing against each other's layout state.
        let [mut popup_engine, mut panel_engine, mut tooltip_engine] =
            [E::default(), E::default(), E::default()];
        let layout_ctx = LayoutContext {
            monitor_id,
            scale: self.monitor_scale(monitor_id),
            style_resolver: self.services.style_resolver.as_ref(),
            child_errors: pass.child_errors,
            current_bounds: current.map(|bounds| bounds.rect()),
            current_constraint: current
                .map_or(SizeConstraint::none(), |bounds| bounds.constraint()),
            current_child_sizes: module_sizes_guard.get(monitor_id),
            interaction_context: Some(interaction_context),
            canvas_factory: &mut self.services.canvas_factory,
            layout_engine: pass.layout_engines.entry(monitor_id.clone()).or_default(),
            popup_layout_engine: &mut popup_engine,
            panel_layout_engine: &mut panel_engine,
            tooltip_layout_engine: &mut tooltip_engine,
        };
        let input = layout_ctx.diff_input(self.services.vdom_diff.as_ref());
        let diff = self.render_pipeline.diff(self.port.as_ref(), input)?;
        self.render_pipeline.layout_and_paint(diff, layout_ctx)
    }

    /// The hovered, pressed and focused nodes on `monitor_id`, and whether Hyprland focuses it.
    fn interaction_on(&self, monitor_id: &MonitorId) -> InteractionContext {
        let monitor_focus = MonitorFocus::from(
            self.ctx
                .hub()
                .hyprland_rx()
                .borrow()
                .focused_monitor()
                .is_some_and(|focused| focused.as_str() == monitor_id.as_str()),
        );
        self.pointer_handler
            .interaction_context(monitor_id, monitor_focus)
    }

    /// The scale Wayland reports for `monitor_id`, 1 when it reports none.
    fn monitor_scale(&self, monitor_id: &MonitorId) -> Scale {
        self.ctx
            .hub()
            .monitor_scales_rx()
            .borrow()
            .get(monitor_id)
            .copied()
            .unwrap_or_else(|| Scale::new(1.0))
    }
}

/// Logs a render pass of module `module` that began at `started`.
fn log_ui_updated(module: ModuleId, started: Instant) {
    tracing::debug!(
        module = %module,
        duration_ms = started.elapsed().as_millis(),
        duration_micros = started.elapsed().as_micros(),
        "Module UI updated"
    );
}
