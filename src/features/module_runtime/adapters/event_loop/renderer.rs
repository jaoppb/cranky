use super::runner::EventLoop;
use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::layout_engine::domain::DisplayCommandSender;
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::module_runtime::domain::LayoutContext;
use crate::features::module_runtime::ports::LayoutEventSender;
use crate::features::vdom::domain::UiCommandSender;
use crate::shared::primitives::geometry::{Rect, Scale};
use crate::shared::primitives::MonitorId;
use crate::shared::rendering::ports::canvas::CanvasFactory;
use std::collections::HashMap;

impl<
    F: CanvasFactory + 'static,
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
> EventLoop<F, LS, DS, US>
{
    pub fn render_all_monitors(
        &mut self,
        layout_engines: &mut HashMap<MonitorId, Box<dyn LayoutEnginePort>>,
    ) {
        let t0 = std::time::Instant::now();
        let layouts: HashMap<MonitorId, Rect> = self.ctx.rxs_mut().0.borrow().clone();
        let monitors = self.discover_monitors(&layouts);

        for monitor_id in monitors {
            let current_bounds = layouts.get(&monitor_id).copied();
            let module_sizes_guard = self.ctx.hub().module_sizes_rx().borrow().clone();
            let current_child_sizes = module_sizes_guard.get(&monitor_id);

            let is_monitor_focused = self
                .ctx
                .hub()
                .hyprland_rx()
                .borrow()
                .focused_monitor()
                .is_some_and(|m| m.as_str() == monitor_id.as_str());
            let interaction_context = Some(
                self.pointer_handler
                    .interaction_context(&monitor_id, is_monitor_focused),
            );

            let engine = layout_engines
                .entry(monitor_id.clone())
                .or_insert_with(|| Box::new(TaffyLayoutAdapter::new()));
            // Fresh per render, never cached: popup/panel content doesn't
            // need incremental diffing the way the bar tree does, and two
            // separate instances keep an open popup and an open panel from
            // diffing against each other's layout state.
            let mut popup_engine = TaffyLayoutAdapter::new();
            let mut panel_engine = TaffyLayoutAdapter::new();
            let mut tooltip_engine = TaffyLayoutAdapter::new();

            let scale = self
                .ctx
                .hub()
                .monitor_scales_rx()
                .borrow()
                .get(&monitor_id)
                .copied()
                .unwrap_or_else(|| Scale::new(1.0));

            let outcome = {
                let layout_ctx = LayoutContext {
                    scale,
                    style_resolver: self.style_resolver.as_ref(),
                    current_bounds,
                    current_child_sizes,
                    interaction_context,
                    canvas_factory: &mut self.canvas_factory,
                    layout_engine: engine.as_mut(),
                    popup_layout_engine: &mut popup_engine,
                    panel_layout_engine: &mut panel_engine,
                    tooltip_layout_engine: &mut tooltip_engine,
                };
                self.render_pipeline.process_monitor(
                    &monitor_id,
                    self.port.as_ref(),
                    self.vdom_diff.as_ref(),
                    layout_ctx,
                )
            };

            if let Some(outcome) = outcome {
                self.dispatch_render_outcome(&monitor_id, &outcome, scale);
            }
        }

        tracing::debug!(
            module = %self.ctx.id(),
            duration_ms = t0.elapsed().as_millis(),
            duration_micros = t0.elapsed().as_micros(),
            "Module UI updated"
        );
    }
}
