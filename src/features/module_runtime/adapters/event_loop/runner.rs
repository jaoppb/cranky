use super::dispatch::{Embedding, discover_monitors};
use super::events::EventLoopEvent;
use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::layout_engine::domain::{DisplayCommandSender, RenderNode};
use crate::features::module_runtime::application::ModuleContext;
use crate::features::module_runtime::domain::{PointerHandler, RenderPipeline, RenderServices};
use crate::features::module_runtime::ports::{AnyModulePort, LayoutEventSender};
use crate::features::vdom::domain::UiCommandSender;
use crate::shared::events::signals::SignalKind;
use crate::shared::primitives::MonitorId;
use crate::shared::primitives::geometry::Rect;
use crate::shared::rendering::ports::canvas::CanvasFactory;
use std::collections::HashMap;
use std::ops::ControlFlow;

pub(in crate::features::module_runtime) struct EventLoop<
    F: CanvasFactory + 'static,
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
> {
    pub(crate) port: Box<dyn AnyModulePort>,
    pub(crate) ctx: ModuleContext<LS, DS, US>,
    pub(crate) pointer_handler: PointerHandler,
    pub(crate) render_pipeline: RenderPipeline,
    pub(crate) services: RenderServices<F>,
    /// Also this module's dedup key for each floating kind: a display
    /// command is only ever sent when a fresh (anchor, layout) pair differs
    /// from what's cached here, so there's one cache per kind, not a
    /// hit-test copy and a separate "did it change" copy. The anchor is part
    /// of the key because a popup can be repositioned (a sibling's width
    /// shifting this module's x-offset) without its content changing at all.
    pub(crate) popup_render_trees: HashMap<MonitorId, (Option<Rect>, RenderNode)>,
    pub(crate) panel_render_trees: HashMap<MonitorId, (Option<Rect>, RenderNode)>,
    pub(crate) tooltip_render_trees: HashMap<MonitorId, (Option<Rect>, RenderNode)>,
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
    /// An event loop for `port` that has handled no pointer event and rendered no monitor yet.
    #[must_use]
    pub(crate) fn new(
        port: Box<dyn AnyModulePort>,
        ctx: ModuleContext<LS, DS, US>,
        services: RenderServices<F>,
    ) -> Self {
        Self {
            port,
            ctx,
            pointer_handler: PointerHandler::new(),
            render_pipeline: RenderPipeline::new(),
            services,
            popup_render_trees: HashMap::new(),
            panel_render_trees: HashMap::new(),
            tooltip_render_trees: HashMap::new(),
            child_layouts_present: HashMap::new(),
        }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn render_pipeline(&self) -> &RenderPipeline {
        &self.render_pipeline
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn into_actor(
        self,
    ) -> crate::features::module_runtime::application::ModuleActor<F, LS, DS, US> {
        crate::features::module_runtime::application::ModuleActor::new(
            self.port,
            self.ctx,
            self.services,
        )
    }

    pub(crate) fn discover_monitors<V>(&self, layouts: &HashMap<MonitorId, V>) -> Vec<MonitorId> {
        let seen: std::collections::HashSet<MonitorId>;
        let embedding = if self.ctx.parent_id().is_some() {
            seen = self
                .render_pipeline
                .render_trees()
                .keys()
                .cloned()
                .collect();
            Embedding::Embedded { seen: &seen }
        } else {
            Embedding::TopLevel
        };
        discover_monitors(self.ctx.hub(), layouts, embedding)
    }

    pub(crate) async fn run(mut self) {
        let subs = self.port.subscriptions().to_vec();
        let mut events_stream = self.ctx.hub().subscribe_streams(&subs);
        let mut layout_engines: HashMap<MonitorId, TaffyLayoutAdapter> = HashMap::new();

        self.port.refresh(self.ctx.hub(), &subs);
        self.render_all_monitors(&mut layout_engines);

        let mut module_sizes_rx = self.ctx.hub().module_sizes_rx();

        loop {
            let event = self
                .poll_next_event(&mut events_stream, &mut module_sizes_rx)
                .await;
            match self.apply_event(event, &subs) {
                ControlFlow::Break(()) => break,
                ControlFlow::Continue(true) => self.render_all_monitors(&mut layout_engines),
                ControlFlow::Continue(false) => (),
            }
        }
    }

    /// Applies one polled event to the module: breaks on shutdown, and otherwise continues with
    /// whether the module must be rendered again.
    fn apply_event(&mut self, event: EventLoopEvent, subs: &[SignalKind]) -> ControlFlow<(), bool> {
        let should_render = match event {
            EventLoopEvent::Shutdown => return ControlFlow::Break(()),
            EventLoopEvent::Signals(sigs) => self.refresh_for(&sigs),
            EventLoopEvent::ModuleSizesChanged => self.child_sizes_changed(),
            EventLoopEvent::LayoutChanged => {
                self.port.refresh(self.ctx.hub(), subs);
                true
            }
            EventLoopEvent::Interaction(monitor_id, interaction) => {
                self.handle_interaction(&monitor_id, interaction)
            }
        };
        ControlFlow::Continue(should_render)
    }

    /// Refreshes the script for `sigs`, if there are any, and tells whether it did.
    fn refresh_for(&mut self, sigs: &[SignalKind]) -> bool {
        if sigs.is_empty() {
            return false;
        }
        self.port.refresh(self.ctx.hub(), sigs);
        true
    }

    /// Whether the child sizes on the hub differ, on any monitor, from those its last render used.
    fn child_sizes_changed(&self) -> bool {
        let current_sizes = self.ctx.hub().module_sizes_rx().borrow().clone();
        self.render_pipeline
            .last_child_sizes()
            .iter()
            .any(|(mon_id, last_sizes)| last_sizes.as_ref() != current_sizes.get(mon_id))
    }
}
