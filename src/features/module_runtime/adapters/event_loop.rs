use crate::app::commands::AppCommand;
use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::module_runtime::application::ModuleContext;
use crate::features::module_runtime::domain::{
    PointerAction, PointerHandler, RenderPipeline,
};
use crate::features::module_runtime::ports::AnyModulePort;
use crate::features::styling::ports::StyleResolverPort;
use crate::features::vdom::ports::VdomDiffPort;
use crate::shared::primitives::geometry::Rect;
use crate::shared::primitives::MonitorId;
use crate::shared::rendering::ports::canvas::CanvasFactory;
use futures_util::StreamExt;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

pub struct EventLoop<F: CanvasFactory + 'static> {
    port: Box<dyn AnyModulePort>,
    ctx: ModuleContext,
    pointer_handler: PointerHandler,
    render_pipeline: RenderPipeline,
    canvas_factory: Arc<Mutex<F>>,
    vdom_diff: Arc<dyn VdomDiffPort>,
    style_resolver: Arc<dyn StyleResolverPort>,
}

impl<F: CanvasFactory + 'static> EventLoop<F> {
    pub fn new(
        port: Box<dyn AnyModulePort>,
        ctx: ModuleContext,
        pointer_handler: PointerHandler,
        render_pipeline: RenderPipeline,
        canvas_factory: Arc<Mutex<F>>,
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

    pub fn render_pipeline(&self) -> &RenderPipeline {
        &self.render_pipeline
    }

    pub fn render_pipeline_mut(&mut self) -> &mut RenderPipeline {
        &mut self.render_pipeline
    }

    pub fn into_actor(self) -> crate::features::module_runtime::application::ModuleActor<F> {
        crate::features::module_runtime::application::ModuleActor::new(
            self.port,
            self.ctx,
            self.canvas_factory,
            self.style_resolver,
            self.vdom_diff,
        )
    }

    pub fn run(mut self) {
        let rt = tokio::runtime::Handle::current();
        let subs = self.port.subscriptions().to_vec();
        let mut events_stream = self.ctx.hub().subscribe_streams(&subs);

        let mut layout_engines: HashMap<MonitorId, Box<dyn LayoutEnginePort>> = HashMap::new();

        // Initial refresh & render
        self.port.refresh(self.ctx.hub(), &subs);
        self.render_all_monitors(&mut layout_engines);

        let mut module_sizes_rx = self.ctx.hub().module_sizes_rx();

        loop {
            let mut changed = false;
            let mut changed_signals = HashSet::new();

            let should_continue = rt.block_on(async {
                let ctx_id = self.ctx.id();
                let (layout_rx, input_rx) = self.ctx.rxs_mut();

                tokio::select! {
                    Some(sig) = events_stream.next(), if !events_stream.is_empty() => {
                        changed = true;
                        changed_signals.insert(sig);
                        while let Some(Some(sig2)) = futures_util::FutureExt::now_or_never(events_stream.next()) {
                            changed_signals.insert(sig2);
                        }
                    }
                    res = module_sizes_rx.changed() => {
                        if res.is_err() {
                            return false;
                        }
                    }
                    res = layout_rx.changed() => {
                        if res.is_err() {
                            return false;
                        }
                        changed = true;
                        for sub in &subs {
                            changed_signals.insert(sub.clone());
                        }
                    }
                    res = input_rx.recv() => {
                        match res {
                            Ok((target_id, monitor_id, event)) => {
                                if target_id == ctx_id {
                                    tracing::debug!(
                                        module = %ctx_id,
                                        monitor = %monitor_id,
                                        event = ?event,
                                        "Received pointer event in module actor"
                                    );
                                    if let Some(render_tree) = self.render_pipeline.render_trees().get(&monitor_id) {
                                        let actions = self.pointer_handler.handle_event(&event, &monitor_id, render_tree);
                                        for action in actions {
                                            match action {
                                                PointerAction::CallFunction(func_name) => {
                                                    if let Err(e) = self.port.call_function(&func_name) {
                                                        tracing::error!(module = %ctx_id, func = %func_name, "ScriptCall failed: {}", e);
                                                    } else {
                                                        changed = true;
                                                    }
                                                }
                                                PointerAction::SendCommand(cmd) => {
                                                    self.ctx.command_tx().send_command(cmd);
                                                }
                                            }
                                        }
                                    } else {
                                        tracing::warn!(
                                            module = %ctx_id,
                                            monitor = %monitor_id,
                                            "Received pointer event but no render tree found for monitor"
                                        );
                                    }
                                }
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                tracing::warn!(module = %ctx_id, lagged = n, "ModuleActor input_rx lagged, skipped messages");
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                tracing::debug!(module = %ctx_id, "ModuleActor input_rx closed");
                                return false;
                            }
                        }
                    }
                }

                true
            });

            if !should_continue {
                break;
            }

            if changed {
                let sigs: Vec<_> = changed_signals.into_iter().collect();
                self.port.refresh(self.ctx.hub(), &sigs);
            }

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

    #[tracing::instrument(level = "debug", skip(self, layout_engines), fields(module = %self.ctx.id()))]
    pub fn render_all_monitors(
        &mut self,
        layout_engines: &mut HashMap<MonitorId, Box<dyn LayoutEnginePort>>,
    ) {
        let t0 = std::time::Instant::now();
        let layouts: HashMap<MonitorId, Rect> = self.ctx.rxs_mut().0.borrow().clone();

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
        let monitors: Vec<MonitorId> = all_monitors.into_iter().collect();

        for monitor_id in monitors {
            let current_bounds = layouts.get(&monitor_id).copied();
            let module_sizes_guard = self.ctx.hub().module_sizes_rx().borrow().clone();
            let current_child_sizes = module_sizes_guard.get(&monitor_id);

            let engine = layout_engines
                .entry(monitor_id.clone())
                .or_insert_with(|| Box::new(TaffyLayoutAdapter::new()));

            let outcome = {
                let mut factory = self.canvas_factory.lock().unwrap();
                let ctx = crate::features::module_runtime::domain::ProcessMonitorContext::new(
                    self.port.as_ref(),
                    self.vdom_diff.as_ref(),
                    self.style_resolver.as_ref(),
                    current_bounds,
                    current_child_sizes,
                    &mut *factory,
                    engine.as_mut(),
                );
                self.render_pipeline.process_monitor(&monitor_id, ctx)
            };

            if let Some(outcome) = outcome {
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
                    let mod_id = self.ctx.id();
                    let parent_id = self.ctx.parent_id();
                    let mon_id = monitor_id.clone();
                    let rt = tokio::runtime::Handle::current();
                    rt.block_on(async move {
                        sm.submit_child_buffer(mod_id, parent_id, mon_id, position, buffer)
                            .await;
                    });
                }
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
