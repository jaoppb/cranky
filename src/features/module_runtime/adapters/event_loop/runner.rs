use super::dispatch::{
    discover_monitors, dispatch_outcome_buffer, dispatch_outcome_layouts, dispatch_outcome_popups,
};
use super::events::EventLoopEvent;
use crate::features::layout_engine::domain::DisplayCommandSender;
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::module_runtime::application::ModuleContext;
use crate::features::module_runtime::domain::{PointerAction, PointerHandler, RenderPipeline};
use crate::features::module_runtime::ports::{AnyModulePort, LayoutEventSender};
use crate::features::styling::ports::StyleResolverPort;
use crate::features::vdom::domain::UiCommandSender;
use crate::features::vdom::ports::VdomDiffPort;
use crate::shared::primitives::geometry::Rect;
use crate::shared::primitives::MonitorId;
use crate::shared::rendering::ports::canvas::CanvasFactory;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub struct EventLoop<
    F: CanvasFactory + 'static,
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
> {
    pub(crate) port: Box<dyn AnyModulePort>,
    pub(crate) ctx: ModuleContext<LS, DS, US>,
    pub(crate) pointer_handler: PointerHandler,
    pub(crate) render_pipeline: RenderPipeline,
    pub(crate) canvas_factory: F,
    pub(crate) vdom_diff: Arc<dyn VdomDiffPort>,
    pub(crate) style_resolver: Arc<dyn StyleResolverPort>,
    pub(crate) active_popups: HashSet<MonitorId>,
}

impl<
    F: CanvasFactory + 'static,
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
> EventLoop<F, LS, DS, US>
{
    #[must_use]
    pub fn new(
        port: Box<dyn AnyModulePort>,
        ctx: ModuleContext<LS, DS, US>,
        pointer_handler: PointerHandler,
        render_pipeline: RenderPipeline,
        canvas_factory: F,
        style_resolver: Arc<dyn StyleResolverPort>,
        vdom_diff: Arc<dyn VdomDiffPort>,
    ) -> Self {
        Self {
            port,
            ctx,
            pointer_handler,
            render_pipeline,
            canvas_factory,
            vdom_diff,
            style_resolver,
            active_popups: HashSet::new(),
        }
    }

    #[must_use]
    pub const fn render_pipeline(&self) -> &RenderPipeline {
        &self.render_pipeline
    }

    pub const fn render_pipeline_mut(&mut self) -> &mut RenderPipeline {
        &mut self.render_pipeline
    }

    #[must_use]
    pub fn into_actor(
        self,
    ) -> crate::features::module_runtime::application::ModuleActor<F, LS, DS, US> {
        crate::features::module_runtime::application::ModuleActor::new(
            self.port,
            self.ctx,
            self.canvas_factory,
            self.style_resolver,
            self.vdom_diff,
        )
    }

    pub fn discover_monitors(&self, layouts: &HashMap<MonitorId, Rect>) -> Vec<MonitorId> {
        discover_monitors(self.ctx.hub(), layouts)
    }

    pub fn dispatch_render_outcome(
        &mut self,
        monitor_id: &MonitorId,
        outcome: &crate::features::module_runtime::domain::RenderOutcome,
    ) {
        dispatch_outcome_layouts(
            self.ctx.layout_sender(),
            self.ctx.id(),
            monitor_id,
            outcome,
        );
        dispatch_outcome_buffer(
            self.ctx.surface_manager(),
            self.ctx.id(),
            self.ctx.parent_id(),
            monitor_id,
            outcome,
        );
        dispatch_outcome_popups(
            self.ctx.display_sender(),
            &mut self.active_popups,
            self.ctx.id(),
            monitor_id,
            outcome,
        );
    }

    pub async fn run(mut self) {
        let subs = self.port.subscriptions().to_vec();
        let mut events_stream = self.ctx.hub().subscribe_streams(&subs);
        let mut layout_engines: HashMap<MonitorId, Box<dyn LayoutEnginePort>> = HashMap::new();

        self.port.refresh(self.ctx.hub(), &subs);
        self.render_all_monitors(&mut layout_engines);

        let mut module_sizes_rx = self.ctx.hub().module_sizes_rx();

        loop {
            let event = self
                .poll_next_event(&mut events_stream, &mut module_sizes_rx)
                .await;
            let mut should_render = false;

            match event {
                EventLoopEvent::Shutdown => break,
                EventLoopEvent::Signals(sigs) => {
                    if !sigs.is_empty() {
                        self.port.refresh(self.ctx.hub(), &sigs);
                        should_render = true;
                    }
                }
                EventLoopEvent::ModuleSizesChanged => {
                    let current_sizes = self.ctx.hub().module_sizes_rx().borrow().clone();
                    for (mon_id, last_sizes) in self.render_pipeline.last_child_sizes() {
                        let current_mon_sizes = current_sizes.get(mon_id);
                        if last_sizes.as_ref() != current_mon_sizes {
                            should_render = true;
                            break;
                        }
                    }
                }
                EventLoopEvent::LayoutChanged => {
                    self.port.refresh(self.ctx.hub(), &subs);
                    should_render = true;
                }
                EventLoopEvent::Pointer(monitor_id, event) => {
                    let changed = self.handle_pointer_event(&monitor_id, &event);
                    if changed {
                        should_render = true;
                    }
                }
            }

            if should_render {
                self.render_all_monitors(&mut layout_engines);

                let post_actions = self
                    .pointer_handler
                    .update_after_render(self.render_pipeline.render_trees());
                for action in post_actions {
                    match action {
                        PointerAction::CallFunction(func_name, mon_id) => {
                            let _ = if let Some(m) = mon_id {
                                self.port.call_function_with_args(&func_name, &[m.as_str()])
                            } else {
                                self.port.call_function(&func_name)
                            };
                        }
                        PointerAction::SendUi(cmd) => {
                            self.ctx.ui_sender().send_ui_command(cmd);
                        }
                        PointerAction::SendDisplay(cmd) => {
                            self.ctx.display_sender().send_display_command(cmd);
                        }
                    }
                }
            }
        }
    }
}
