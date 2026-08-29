use crate::app::commands::AppCommand;
use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::module_runtime::application::ModuleContext;
use crate::features::module_runtime::domain::{PointerAction, PointerHandler, RenderPipeline};
use crate::features::module_runtime::ports::AnyModulePort;
use crate::features::styling::ports::StyleResolverPort;
use crate::features::vdom::ports::VdomDiffPort;
use crate::shared::events::signals::SignalKind;
use crate::shared::primitives::MonitorId;
use crate::shared::primitives::geometry::Rect;
use crate::shared::rendering::ports::canvas::CanvasFactory;
use futures_util::StreamExt;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Debug, PartialEq, Clone)]
pub enum EventLoopEvent {
    Signals(Vec<SignalKind>),
    ModuleSizesChanged,
    LayoutChanged,
    Pointer(MonitorId, crate::shared::events::core::PointerEvent),
    Shutdown,
}

pub struct EventLoop<F: CanvasFactory + 'static> {
    port: Box<dyn AnyModulePort>,
    ctx: ModuleContext,
    pointer_handler: PointerHandler,
    render_pipeline: RenderPipeline,
    canvas_factory: F,
    vdom_diff: Arc<dyn VdomDiffPort>,
    style_resolver: Arc<dyn StyleResolverPort>,
}

impl<F: CanvasFactory + 'static> EventLoop<F> {
    #[must_use]
    pub fn new(
        port: Box<dyn AnyModulePort>,
        ctx: ModuleContext,
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
    pub fn into_actor(self) -> crate::features::module_runtime::application::ModuleActor<F> {
        crate::features::module_runtime::application::ModuleActor::new(
            self.port,
            self.ctx,
            self.canvas_factory,
            self.style_resolver,
            self.vdom_diff,
        )
    }

    pub async fn poll_next_event(
        &mut self,
        events_stream: &mut futures_util::stream::SelectAll<
            futures_util::stream::BoxStream<'static, SignalKind>,
        >,
        module_sizes_rx: &mut tokio::sync::watch::Receiver<
            HashMap<MonitorId, crate::shared::primitives::ChildSizesMap>,
        >,
    ) -> EventLoopEvent {
        let ctx_id = self.ctx.id();
        let (layout_rx, input_rx) = self.ctx.rxs_mut();

        tokio::select! {
            Some(sig) = events_stream.next(), if !events_stream.is_empty() => {
                let mut changed_signals = HashSet::new();
                changed_signals.insert(sig);
                while let Some(Some(sig2)) = futures_util::FutureExt::now_or_never(events_stream.next()) {
                    changed_signals.insert(sig2);
                }
                EventLoopEvent::Signals(changed_signals.into_iter().collect())
            }
            res = module_sizes_rx.changed() => {
                if res.is_err() {
                    EventLoopEvent::Shutdown
                } else {
                    EventLoopEvent::ModuleSizesChanged
                }
            }
            res = layout_rx.changed() => {
                if res.is_err() {
                    EventLoopEvent::Shutdown
                } else {
                    EventLoopEvent::LayoutChanged
                }
            }
            res = input_rx.recv() => {
                match res {
                    Ok((target_id, monitor_id, event)) => {
                        if target_id == ctx_id {
                            EventLoopEvent::Pointer(monitor_id, event)
                        } else {
                            EventLoopEvent::Signals(Vec::new())
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(module = %ctx_id, lagged = n, "ModuleActor input_rx lagged, skipped messages");
                        EventLoopEvent::Signals(Vec::new())
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::debug!(module = %ctx_id, "ModuleActor input_rx closed");
                        EventLoopEvent::Shutdown
                    }
                }
            }
        }
    }

    pub fn handle_pointer_event(
        &mut self,
        monitor_id: &MonitorId,
        event: &crate::shared::events::core::PointerEvent,
    ) -> bool {
        let ctx_id = self.ctx.id();
        tracing::debug!(
            module = %ctx_id,
            monitor = %monitor_id,
            event = ?event,
            "Received pointer event in module actor"
        );
        if let Some(render_tree) = self.render_pipeline.render_trees().get(monitor_id) {
            let actions = self
                .pointer_handler
                .handle_event(event, monitor_id, render_tree);
            let mut changed = false;
            for action in actions {
                match action {
                    PointerAction::CallFunction(func_name) => {
                        if let Err(e) = self.port.call_function(&func_name) {
                            tracing::error!(
                                module = %ctx_id,
                                func = %func_name,
                                "ScriptCall failed: {e}"
                            );
                        } else {
                            changed = true;
                        }
                    }
                    PointerAction::SendCommand(cmd) => {
                        self.ctx.command_tx().send_command(cmd);
                    }
                }
            }
            if changed {
                let subs = self.port.subscriptions().to_vec();
                self.port.refresh(self.ctx.hub(), &subs);
            }
            changed
        } else {
            tracing::warn!(
                module = %ctx_id,
                monitor = %monitor_id,
                "Received pointer event but no render tree found for monitor"
            );
            false
        }
    }

    pub async fn run(mut self) {
        let subs = self.port.subscriptions().to_vec();
        let mut events_stream = self.ctx.hub().subscribe_streams(&subs);
        let mut layout_engines: HashMap<MonitorId, Box<dyn LayoutEnginePort>> = HashMap::new();

        // Initial refresh & render
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
                    if let PointerAction::SendCommand(cmd) = action {
                        self.ctx.command_tx().send_command(cmd);
                    }
                }
            }
        }
    }

    #[must_use]
    pub fn discover_monitors(&self, layouts: &HashMap<MonitorId, Rect>) -> Vec<MonitorId> {
        let mut all_monitors: HashSet<MonitorId> = HashSet::new();
        for m in self.ctx.hub().hyprland_rx().borrow().monitors().values() {
            all_monitors.insert(MonitorId::new(m.name().as_str()));
        }
        for m in layouts.keys() {
            all_monitors.insert(m.clone());
        }
        for m in self.ctx.hub().module_sizes_rx().borrow().keys() {
            all_monitors.insert(m.clone());
        }
        all_monitors.into_iter().collect()
    }

    pub fn dispatch_render_outcome(
        &mut self,
        monitor_id: &MonitorId,
        outcome: crate::features::module_runtime::domain::RenderOutcome,
    ) {
        if !outcome.child_layouts().is_empty() {
            self.ctx
                .command_tx()
                .send_command(AppCommand::ContainerLayoutsCalculated {
                    parent_id: self.ctx.id(),
                    monitor_id: monitor_id.clone(),
                    layouts: outcome.child_layouts().to_vec(),
                });
        }

        if let Some(size_change) = outcome.size_change() {
            self.ctx
                .command_tx()
                .send_command(AppCommand::ModuleSizeChanged(
                    monitor_id.clone(),
                    self.ctx.id(),
                    size_change.new_size(),
                ));
        }

        if let Some((buffer, position)) = outcome.into_buffer() {
            let sm = self.ctx.surface_manager().clone();
            let module_id = self.ctx.id();
            let parent_id = self.ctx.parent_id();
            let target_monitor_id = monitor_id.clone();
            sm.submit_child_buffer(module_id, parent_id, target_monitor_id, position, buffer);
        }
    }

    #[tracing::instrument(level = "debug", skip(self, layout_engines), fields(module = %self.ctx.id()))]
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

            let engine = layout_engines
                .entry(monitor_id.clone())
                .or_insert_with(|| Box::new(TaffyLayoutAdapter::new()));

            let outcome = {
                let layout_ctx = crate::features::module_runtime::domain::LayoutContext {
                    style_resolver: self.style_resolver.as_ref(),
                    current_bounds,
                    current_child_sizes,
                    canvas_factory: &mut self.canvas_factory,
                    layout_engine: engine.as_mut(),
                };
                self.render_pipeline.process_monitor(
                    &monitor_id,
                    self.port.as_ref(),
                    self.vdom_diff.as_ref(),
                    layout_ctx,
                )
            };

            if let Some(outcome) = outcome {
                self.dispatch_render_outcome(&monitor_id, outcome);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::module_runtime::test_support::{
        MockCanvasFactory, MockCommandSender, MockSurfaceManager, TestModulePort,
    };
    use crate::features::styling::adapters::fs_loader::CompositeStyleResolver;
    use crate::features::vdom::adapters::DefaultVdomDiffAdapter;
    use crate::features::vdom::domain::VNode;
    use crate::shared::config::domain::Config;
    use crate::shared::events::signals::SignalHub;
    use crate::shared::primitives::ModuleId;
    use crate::shared::primitives::geometry::{Position, Size};

    #[test]
    fn test_discover_monitors_aggregates_sources() {
        let id = ModuleId::new(1);
        let hub = Arc::new(SignalHub::new(Config::default()));
        let sm = Arc::new(MockSurfaceManager);
        let cmd = Arc::new(MockCommandSender);
        let (_tx, rx) = tokio::sync::watch::channel(HashMap::new());
        let ctx = ModuleContext::new(id, hub, sm, cmd, rx);

        let event_loop = EventLoop::new(
            Box::new(TestModulePort::new(VNode::new_rect(
                None, None, None, None, None,
            ))),
            ctx,
            PointerHandler::new(),
            RenderPipeline::new(),
            MockCanvasFactory,
            Arc::new(CompositeStyleResolver::new(vec![])),
            Arc::new(DefaultVdomDiffAdapter::new()),
        );

        let mut layouts = HashMap::new();
        layouts.insert(
            MonitorId::new("DP-1"),
            Rect::new(Position::new(0, 0), Size::new(100, 30)),
        );

        let monitors = event_loop.discover_monitors(&layouts);
        assert!(monitors.contains(&MonitorId::new("DP-1")));
    }

    #[tokio::test]
    async fn test_dispatch_render_outcome_sends_commands() {
        use crate::features::module_runtime::domain::SizeChange;
        use crate::features::module_runtime::test_support::ChannelCommandSender;
        use crate::features::styling::domain::ComputedStyle;

        let id = ModuleId::new(1);
        let hub = Arc::new(SignalHub::new(Config::default()));
        let sm = Arc::new(MockSurfaceManager);
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();
        let cmd = Arc::new(ChannelCommandSender::new(cmd_tx));
        let (_tx, rx) = tokio::sync::watch::channel(HashMap::new());
        let ctx = ModuleContext::new(id, hub, sm, cmd, rx);

        let mut event_loop = EventLoop::new(
            Box::new(TestModulePort::new(VNode::new_rect(
                None, None, None, None, None,
            ))),
            ctx,
            PointerHandler::new(),
            RenderPipeline::new(),
            MockCanvasFactory,
            Arc::new(CompositeStyleResolver::new(vec![])),
            Arc::new(DefaultVdomDiffAdapter::new()),
        );

        let outcome = crate::features::module_runtime::domain::RenderOutcome::new(
            Some(SizeChange::new(Size::new(0, 0), Size::new(50, 20))),
            vec![],
            crate::features::layout_engine::domain::RenderNode::Rect {
                rect: Rect::new(Position::new(0, 0), Size::new(50, 20)),
                style: ComputedStyle::default(),
                on_click: None,
                on_hover: None,
                tooltip: None,
            },
            None,
        );

        event_loop.dispatch_render_outcome(&MonitorId::new("DP-1"), outcome);

        let received = cmd_rx
            .try_recv()
            .expect("Should have sent ModuleSizeChanged");
        match received {
            AppCommand::ModuleSizeChanged(mon, mod_id, size) => {
                assert_eq!(mon.as_str(), "DP-1");
                assert_eq!(mod_id, id);
                assert_eq!(size, Size::new(50, 20));
            }
            _ => panic!("Unexpected command"),
        }
    }

    #[test]
    fn test_handle_pointer_event_no_tree_returns_false() {
        let id = ModuleId::new(1);
        let hub = Arc::new(SignalHub::new(Config::default()));
        let sm = Arc::new(MockSurfaceManager);
        let cmd = Arc::new(MockCommandSender);
        let (_tx, rx) = tokio::sync::watch::channel(HashMap::new());
        let ctx = ModuleContext::new(id, hub, sm, cmd, rx);

        let mut event_loop = EventLoop::new(
            Box::new(TestModulePort::new(VNode::new_rect(
                None, None, None, None, None,
            ))),
            ctx,
            PointerHandler::new(),
            RenderPipeline::new(),
            MockCanvasFactory,
            Arc::new(CompositeStyleResolver::new(vec![])),
            Arc::new(DefaultVdomDiffAdapter::new()),
        );

        let event = crate::shared::events::core::PointerEvent::Click {
            button: 0,
            x: 10.0,
            y: 10.0,
        };
        let changed = event_loop.handle_pointer_event(&MonitorId::new("DP-1"), &event);
        assert!(!changed);
    }
}
