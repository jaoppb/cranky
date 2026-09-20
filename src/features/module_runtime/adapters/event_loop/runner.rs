use super::dispatch::discover_monitors;
use super::events::EventLoopEvent;
use crate::features::layout_engine::domain::{DisplayCommandSender, RenderNode};
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
    /// Also this module's dedup key for each floating kind: a display
    /// command is only ever sent when a fresh layout differs from what's
    /// cached here, so there's one cache per kind, not a hit-test copy and a
    /// separate "did it change" copy.
    pub(crate) popup_render_trees: HashMap<MonitorId, RenderNode>,
    pub(crate) panel_render_trees: HashMap<MonitorId, RenderNode>,
    pub(crate) tooltip_render_trees: HashMap<MonitorId, RenderNode>,
    /// Whether the last render on each monitor had any child modules at all
    /// — lets `dispatch_outcome_layouts` send one final empty
    /// `ContainerLayoutsCalculated` on the render where the last child
    /// disappears, without spamming one on every render of a module that
    /// never has children.
    pub(crate) child_layouts_present: HashMap<MonitorId, bool>,
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
            popup_render_trees: HashMap::new(),
            panel_render_trees: HashMap::new(),
            tooltip_render_trees: HashMap::new(),
            child_layouts_present: HashMap::new(),
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

    pub fn discover_monitors<V>(&self, layouts: &HashMap<MonitorId, V>) -> Vec<MonitorId> {
        let seen: std::collections::HashSet<MonitorId> = self
            .render_pipeline
            .render_trees()
            .keys()
            .cloned()
            .collect();
        discover_monitors(self.ctx.hub(), layouts, &seen)
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
            }
        }
    }
}
