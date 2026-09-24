use super::dispatch::{
    discover_monitors, dispatch_outcome_buffer, dispatch_outcome_layouts, dispatch_outcome_panels,
    dispatch_outcome_popups, dispatch_post_actions, update_floating_trees,
};
use super::events::EventLoopEvent;
use super::floating_state::FloatingState;
use crate::features::layout_engine::domain::DisplayCommandSender;
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::module_runtime::application::ModuleContext;
use crate::features::module_runtime::domain::{PointerHandler, RenderPipeline};
use crate::features::module_runtime::ports::{AnyModulePort, LayoutEventSender};
use crate::features::styling::ports::StyleResolverPort;
use crate::features::vdom::domain::UiCommandSender;
use crate::features::vdom::ports::VdomDiffPort;
use crate::shared::primitives::MonitorId;
use crate::shared::rendering::ports::canvas::CanvasFactory;
use std::collections::HashMap;
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
    pub(super) floating: FloatingState,
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
            floating: FloatingState::default(),
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

    pub fn discover_monitors(&self) -> Vec<MonitorId> {
        discover_monitors(self.ctx.hub())
    }

    pub fn dispatch_render_outcome(
        &mut self,
        monitor_id: &MonitorId,
        outcome: &crate::features::module_runtime::domain::RenderOutcome,
    ) {
        dispatch_outcome_layouts(self.ctx.layout_sender(), self.ctx.id(), monitor_id, outcome);
        dispatch_outcome_buffer(
            self.ctx.surface_manager(),
            self.ctx.id(),
            self.ctx.parent_id(),
            monitor_id,
            outcome,
        );
        dispatch_outcome_popups(
            self.ctx.display_sender(),
            self.floating.active_popups_mut(),
            self.ctx.id(),
            monitor_id,
            outcome,
        );
        dispatch_outcome_panels(
            self.ctx.display_sender(),
            self.floating.active_panels_mut(),
            self.ctx.id(),
            monitor_id,
            outcome,
        );
        let (popup_trees, panel_trees) = self.floating.render_trees_mut();
        update_floating_trees(
            &mut self.canvas_factory,
            popup_trees,
            panel_trees,
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
                        if last_sizes.as_ref() != current_sizes.get(mon_id) {
                            should_render = true;
                            break;
                        }
                    }
                }
                EventLoopEvent::LayoutChanged => {
                    self.port.refresh(self.ctx.hub(), &subs);
                    should_render = true;
                }
                EventLoopEvent::Interaction(monitor_id, interaction) => {
                    let changed = match interaction {
                        crate::shared::events::core::InteractionEvent::Pointer(ev) => {
                            self.handle_pointer_event(&monitor_id, &ev)
                        }
                        crate::shared::events::core::InteractionEvent::Lifecycle(ev) => {
                            self.handle_lifecycle_event(&monitor_id, ev)
                        }
                    };
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
                dispatch_post_actions(&mut self.port, &self.ctx, post_actions);
            }
        }
    }
}
