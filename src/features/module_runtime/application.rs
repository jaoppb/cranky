use crate::app::commands::AppCommand;
use crate::features::module_runtime::ports::AnyModulePort;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::primitives::{
    MonitorId,
    geometry::{Rect, Scale, Size},
};
use crate::shared::wayland::ports::DynSurfaceManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::watch;

pub struct ModuleContext {
    id: crate::shared::primitives::ModuleId,
    hub: Arc<SignalHub>,
    surface_manager: DynSurfaceManager,
    command_tx: Arc<dyn crate::features::module_runtime::ports::CommandSender>,
    layout_rx: watch::Receiver<HashMap<MonitorId, Rect>>,
    pointer_rx: crate::shared::events::core::PointerReceiver,
}

impl ModuleContext {
    pub fn new(
        id: crate::shared::primitives::ModuleId,
        hub: Arc<SignalHub>,
        surface_manager: DynSurfaceManager,
        command_tx: Arc<dyn crate::features::module_runtime::ports::CommandSender>,
        layout_rx: watch::Receiver<HashMap<MonitorId, Rect>>,
    ) -> Self {
        let pointer_rx = hub.pointer_rx();
        Self {
            id,
            hub,
            surface_manager,
            command_tx,
            layout_rx,
            pointer_rx,
        }
    }

    pub fn id(&self) -> crate::shared::primitives::ModuleId {
        self.id
    }

    pub fn hub(&self) -> &Arc<SignalHub> {
        &self.hub
    }

    pub fn surface_manager(&self) -> &DynSurfaceManager {
        &self.surface_manager
    }

    pub fn command_tx(&self) -> &dyn crate::features::module_runtime::ports::CommandSender {
        self.command_tx.as_ref()
    }

    pub fn rxs_mut(
        &mut self,
    ) -> (
        &mut watch::Receiver<HashMap<MonitorId, Rect>>,
        &mut crate::shared::events::core::PointerReceiver,
    ) {
        (&mut self.layout_rx, &mut self.pointer_rx)
    }
}

pub struct ModuleActor<F: crate::shared::rendering::ports::canvas::CanvasFactory + 'static> {
    port: Box<dyn AnyModulePort>,
    ctx: ModuleContext,
    sizes: std::collections::HashMap<MonitorId, Size>,
    canvas_factory: std::sync::Arc<std::sync::Mutex<F>>,
    render_trees:
        std::collections::HashMap<MonitorId, crate::features::layout_engine::domain::RenderNode>,
    style_resolver: std::sync::Arc<dyn crate::features::styling::ports::StyleResolverPort>,
}

impl<F: crate::shared::rendering::ports::canvas::CanvasFactory + 'static> ModuleActor<F> {
    pub fn new(
        port: Box<dyn AnyModulePort>,
        ctx: ModuleContext,
        canvas_factory: std::sync::Arc<std::sync::Mutex<F>>,
        style_resolver: std::sync::Arc<dyn crate::features::styling::ports::StyleResolverPort>,
    ) -> Self {
        Self {
            port,
            ctx,
            sizes: std::collections::HashMap::new(),
            canvas_factory,
            render_trees: std::collections::HashMap::new(),
            style_resolver,
        }
    }

    pub fn spawn(mut self) {
        tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Handle::current();

            use crate::shared::events::signals::SignalKind;
            use futures_util::stream::{SelectAll, StreamExt};
            use tokio_stream::wrappers::WatchStream;

            let mut events_stream = SelectAll::new();
            let subs = self.port.subscriptions();

            if subs.contains(&SignalKind::Time) {
                events_stream.push(
                    WatchStream::new(self.ctx.hub().time_rx())
                        .map(|_| SignalKind::Time)
                        .boxed(),
                );
            }
            if subs.contains(&SignalKind::Hyprland) {
                events_stream.push(
                    WatchStream::new(self.ctx.hub().hyprland_rx())
                        .map(|_| SignalKind::Hyprland)
                        .boxed(),
                );
            }
            if subs.contains(&SignalKind::Applets) {
                events_stream.push(
                    WatchStream::new(self.ctx.hub().applets_rx())
                        .map(|_| SignalKind::Applets)
                        .boxed(),
                );
            }
            if subs.contains(&SignalKind::Metrics) {
                events_stream.push(
                    WatchStream::new(self.ctx.hub().metrics_rx())
                        .map(|_| SignalKind::Metrics)
                        .boxed(),
                );
            }
            if subs.contains(&SignalKind::Mpris) {
                events_stream.push(
                    WatchStream::new(self.ctx.hub().mpris_rx())
                        .map(|_| SignalKind::Mpris)
                        .boxed(),
                );
            }
            if subs.iter().any(|s| matches!(s, SignalKind::DBus(_))) {
                events_stream.push(
                    WatchStream::new(self.ctx.hub().dbus_rx())
                        .map(|_| {
                            SignalKind::DBus(crate::shared::dbus::domain::DBusSubscription::new(
                                crate::shared::dbus::domain::BusType::Session,
                                None,
                                None,
                                None,
                                None,
                            ))
                        })
                        .boxed(),
                );
            }

            let mut layout_engines: std::collections::HashMap<
                MonitorId,
                Box<dyn crate::features::layout_engine::ports::LayoutEnginePort>,
            > = std::collections::HashMap::new();

            // Initial refresh
            let initial_sigs = self.port.subscriptions().to_vec();
            self.port.refresh(self.ctx.hub(), &initial_sigs);
            self.measure_and_render_all(&mut layout_engines);

            let mut last_tooltip: Option<crate::features::layout_engine::domain::StyledNode> = None;
            let mut last_pointer_pos: Option<(
                MonitorId,
                crate::shared::primitives::geometry::Position,
            )> = None;

            loop {
                // Determine what woke us up
                let mut changed = false;
                let mut changed_signals = std::collections::HashSet::new();

                let should_continue = rt.block_on(async {
                    let ctx_id = self.ctx.id();
                    let (layout_rx, input_rx) = self.ctx.rxs_mut();

                    tokio::select! {
                        Some(sig) = events_stream.next(), if !events_stream.is_empty() => {
                            changed = true;
                            changed_signals.insert(sig);
                            // Drain any immediately pending events from the select_all stream to debounce
                            while let Some(Some(sig2)) = futures_util::FutureExt::now_or_never(events_stream.next()) {
                                changed_signals.insert(sig2);
                            }
                        }
                        res = layout_rx.changed() => {
                            if res.is_err() {
                                return false; // layout_rx dropped, we should exit
                            }
                        }
                        res = input_rx.recv() => {
                            match res {
                                Ok((target_id, monitor_id, event)) => {
                                    if target_id == ctx_id {
                                        tracing::debug!(module = %ctx_id, monitor = %monitor_id, event = ?event, "Received pointer event in module actor");
                                        if let Some(render_tree) = self.render_trees.get(&monitor_id) {
                                            use crate::shared::primitives::geometry::Position;
                                            use crate::shared::events::core::PointerEvent;
                                            match event {
                                                PointerEvent::Click { x, y, .. } => {
                                                    let pos = Position::new(x as i32, y as i32);
                                                    last_pointer_pos = Some((monitor_id.clone(), pos));
                                                    let hit = render_tree.hit_test(pos);
                                                    let hit_cmd = hit.iter().rev().find_map(|n| n.on_click());
                                                    tracing::debug!(
                                                        module = %ctx_id,
                                                        monitor = %monitor_id,
                                                        pos = ?pos,
                                                        hit_nodes = hit.len(),
                                                        has_on_click = hit_cmd.is_some(),
                                                        "Hit test for Click"
                                                    );
                                                    if let Some(cmd) = hit_cmd {
                                                        tracing::debug!(module = %ctx_id, cmd = ?cmd, "Sending on_click command");
                                                        if let crate::app::commands::AppCommand::ScriptCall(func_name) = &cmd {
                                                            if let Err(e) = self.port.call_function(func_name) {
                                                                tracing::error!(module = %ctx_id, func = %func_name, "ScriptCall failed: {}", e);
                                                            } else {
                                                                changed = true;
                                                            }
                                                        } else {
                                                            self.ctx.command_tx().send_command(cmd.clone());
                                                        }
                                                    } else {
                                                        tracing::debug!(module = %ctx_id, "No on_click command found in hit tree");
                                                    }
                                                },
                                                PointerEvent::PointerMotion { x, y } => {
                                                    let pos = Position::new(x as i32, y as i32);
                                                    last_pointer_pos = Some((monitor_id.clone(), pos));
                                                    let hit = render_tree.hit_test(pos);
                                                    let hit_cmd = hit.iter().rev().find_map(|n| n.on_hover());
                                                    tracing::debug!(
                                                        module = %ctx_id,
                                                        monitor = %monitor_id,
                                                        pos = ?pos,
                                                        hit_nodes = hit.len(),
                                                        has_on_hover = hit_cmd.is_some(),
                                                        "Hit test for PointerMotion"
                                                    );
                                                    if let Some(cmd) = hit_cmd {
                                                        tracing::debug!(module = %ctx_id, cmd = ?cmd, "Sending on_hover command");
                                                        self.ctx.command_tx().send_command(cmd.clone());
                                                    }

                                                    let hit_tooltip = hit.iter().rev().find_map(|n| n.tooltip()).cloned();
                                                    if hit_tooltip != last_tooltip {
                                                        tracing::debug!(
                                                            module = %ctx_id,
                                                            monitor = %monitor_id,
                                                            changed = true,
                                                            has_tooltip = hit_tooltip.is_some(),
                                                            "Tooltip state changed"
                                                        );
                                                        if let Some(layout) = &hit_tooltip {
                                                            tracing::debug!(module = %ctx_id, ?layout, "Sending ShowTooltip command");
                                                            self.ctx.command_tx().send_command(crate::app::commands::AppCommand::ShowTooltip { layout: Box::new(layout.clone()) });
                                                        } else {
                                                            tracing::debug!(module = %ctx_id, "Sending HideTooltip command");
                                                            self.ctx.command_tx().send_command(crate::app::commands::AppCommand::HideTooltip);
                                                        }
                                                        last_tooltip = hit_tooltip;
                                                    }
                                                },
                                                PointerEvent::PointerEnter => {
                                                    tracing::debug!(module = %ctx_id, "Pointer entered module bounds");
                                                },
                                                PointerEvent::PointerLeave => {
                                                    tracing::debug!(module = %ctx_id, "Pointer left module bounds");
                                                    last_pointer_pos = None;
                                                    if last_tooltip.is_some() {
                                                        tracing::debug!(module = %ctx_id, "Hiding tooltip due to pointer leave");
                                                        self.ctx.command_tx().send_command(crate::app::commands::AppCommand::HideTooltip);
                                                        last_tooltip = None;
                                                    }
                                                },
                                                _ => {}
                                            }
                                        } else {
                                            tracing::warn!(module = %ctx_id, monitor = %monitor_id, "Received pointer event but no render tree found for monitor");
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

                self.measure_and_render_all(&mut layout_engines);

                if let Some((monitor_id, pos)) = &last_pointer_pos
                    && let Some(render_tree) = self.render_trees.get(monitor_id)
                {
                    let hit = render_tree.hit_test(*pos);
                    let hit_tooltip = hit.iter().rev().find_map(|n| n.tooltip()).cloned();
                    if hit_tooltip != last_tooltip {
                        tracing::debug!(module = %self.ctx.id(), changed = true, has_tooltip = hit_tooltip.is_some(), "Tooltip state changed after render");
                        if let Some(layout) = &hit_tooltip {
                            self.ctx.command_tx().send_command(
                                crate::app::commands::AppCommand::ShowTooltip {
                                    layout: Box::new(layout.clone()),
                                },
                            );
                        } else {
                            self.ctx
                                .command_tx()
                                .send_command(crate::app::commands::AppCommand::HideTooltip);
                        }
                        last_tooltip = hit_tooltip;
                    }
                }
            }
        });
    }

    #[tracing::instrument(level = "debug", skip(self, layout_engines), fields(module = %self.ctx.id()))]
    fn measure_and_render_all(
        &mut self,
        layout_engines: &mut std::collections::HashMap<
            MonitorId,
            Box<dyn crate::features::layout_engine::ports::LayoutEnginePort>,
        >,
    ) {
        let t0 = std::time::Instant::now();
        let layouts: std::collections::HashMap<MonitorId, Rect> =
            self.ctx.rxs_mut().0.borrow().clone();

        let mut all_monitors: std::collections::HashSet<MonitorId> =
            std::collections::HashSet::new();
        for m in self.ctx.hub().hyprland_rx().borrow().monitors().values() {
            all_monitors.insert(MonitorId::new(m.name().as_str()));
        }
        for m in layouts.keys() {
            all_monitors.insert(m.clone());
        }
        let monitors: Vec<MonitorId> = all_monitors.into_iter().collect();

        for monitor_id in monitors {
            let layout_node = self.port.render(&monitor_id);
            tracing::trace!(module = %self.ctx.id(), monitor = %monitor_id, "Resolving styles for module layout node");
            let styled_node = layout_node.resolve_styles(self.style_resolver.as_ref(), None);

            // Measure
            let config = self.ctx.hub().config_rx().borrow().clone();
            let default_font_family = config.bar().font_family().clone();
            let default_font_size = config.bar().font_size();

            let render_node_res = {
                let mut factory = self.canvas_factory.lock().unwrap();
                let mut measurer = factory.create_text_measurer(
                    Scale::new(1.0),
                    default_font_family.clone(),
                    default_font_size,
                );

                let engine = layout_engines.entry(monitor_id.clone()).or_insert_with(|| {
                    Box::new(
                        crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter::new(),
                    )
                });

                engine.calculate_layout(
                    styled_node,
                    &mut measurer,
                    crate::shared::primitives::geometry::Position::new(0, 0),
                )
            };

            let render_node = match render_node_res {
                Ok(node) => node,
                Err(e) => {
                    tracing::error!(module = %self.ctx.id(), monitor = %monitor_id, err = ?e, "Module layout calculation failed");
                    continue;
                }
            };

            self.render_trees
                .insert(monitor_id.clone(), render_node.clone());

            let size = *render_node.rect().size();

            let old_size = self
                .sizes
                .get(&monitor_id)
                .copied()
                .unwrap_or(Size::new(0, 0));
            if size != old_size {
                self.sizes.insert(monitor_id.clone(), size);
                self.ctx
                    .command_tx()
                    .send_command(AppCommand::ModuleSizeChanged(
                        monitor_id.clone(),
                        self.ctx.id(),
                        size,
                    ));
            }

            // Render if we have bounds
            if let Some(bounds) = layouts.get(&monitor_id)
                && bounds.width() > 0
                && bounds.height() > 0
            {
                let w = bounds.width();
                let h = bounds.height();
                let mut data = vec![0u8; (w * h * 4) as usize];
                {
                    let mut factory = self.canvas_factory.lock().unwrap();
                    let mut canvas = factory.create_canvas(
                        &mut data,
                        *bounds.size(),
                        Scale::new(1.0),
                        default_font_family.clone(),
                        default_font_size,
                    );
                    render_node.render_to_canvas(&mut canvas);
                }

                let buffer = RenderBuffer::new(data, *bounds.size());
                let rt = tokio::runtime::Handle::current();
                let sm = self.ctx.surface_manager().clone();
                let mod_id = self.ctx.id();
                let mon_id = monitor_id.clone();
                let position =
                    crate::shared::primitives::geometry::Position::new(bounds.x(), bounds.y());
                rt.block_on(async move {
                    sm.submit_buffer(mod_id, mon_id, position, buffer).await;
                });
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
    use crate::features::module_runtime::ports::CommandSender;
    use crate::shared::config::domain::Config;

    struct MockCommandSender;
    impl CommandSender for MockCommandSender {
        fn send_command(&self, _cmd: AppCommand) {}
    }

    struct MockSurfaceManager;
    #[async_trait::async_trait]
    impl crate::shared::wayland::ports::SurfaceManagerPort for MockSurfaceManager {
        async fn submit_buffer(
            &self,
            _mod_id: crate::shared::primitives::ModuleId,
            _mon_id: MonitorId,
            _pos: crate::shared::primitives::geometry::Position,
            _buf: RenderBuffer,
        ) {
        }
    }

    struct TestCommandSender {
        tx: std::sync::mpsc::Sender<AppCommand>,
    }
    impl CommandSender for TestCommandSender {
        fn send_command(&self, cmd: AppCommand) {
            let _ = self.tx.send(cmd);
        }
    }

    struct MockCanvasFactory;

    impl crate::shared::rendering::ports::canvas::CanvasFactory for MockCanvasFactory {
        fn create_canvas<'a>(
            &'a mut self,
            _data: &'a mut [u8],
            _size: Size,
            _scale: Scale,
            _font_family: crate::shared::config::domain::FontFamily,
            _font_size: crate::shared::config::domain::FontSize,
        ) -> impl crate::shared::rendering::ports::canvas::Canvas + 'a {
            MockCanvas
        }

        fn create_text_measurer<'a>(
            &'a mut self,
            _scale: Scale,
            _font_family: crate::shared::config::domain::FontFamily,
            _font_size: crate::shared::config::domain::FontSize,
        ) -> impl crate::features::layout_engine::domain::TextMeasurer + 'a {
            MockMeasurer
        }
    }

    struct MockCanvas;
    impl crate::shared::rendering::ports::canvas::Canvas for MockCanvas {
        fn draw_rect(
            &mut self,
            _x: crate::shared::primitives::geometry::LogicalPx,
            _y: crate::shared::primitives::geometry::LogicalPx,
            _w: crate::shared::primitives::geometry::LogicalPx,
            _h: crate::shared::primitives::geometry::LogicalPx,
            _color: crate::shared::primitives::color::DrawingColor,
            _radius: crate::shared::primitives::geometry::LogicalPx,
        ) {
        }
        fn draw_border(
            &mut self,
            _pos: crate::shared::primitives::geometry::Position,
            _size: crate::shared::primitives::geometry::Size,
            _color: crate::shared::primitives::color::DrawingColor,
            _radius: crate::shared::primitives::geometry::LogicalPx,
            _border_size: crate::shared::primitives::geometry::LogicalPx,
        ) {
        }
        fn draw_text(
            &mut self,
            _text: &str,
            _font_family: Option<&crate::shared::config::domain::FontFamily>,
            _font_size: Option<crate::shared::config::domain::FontSize>,
            _color: crate::shared::primitives::color::DrawingColor,
            _pos: crate::shared::primitives::geometry::Position,
        ) {
        }
        fn draw_image(
            &mut self,
            _image_data: &[u8],
            _pixel_size: crate::shared::primitives::geometry::Size,
            _logical_size: crate::shared::primitives::geometry::Size,
            _pos: crate::shared::primitives::geometry::Position,
        ) {
        }
    }

    struct MockMeasurer;
    impl crate::features::layout_engine::domain::TextMeasurer for MockMeasurer {
        fn measure(
            &mut self,
            _text: &str,
            _font_family: Option<&crate::shared::config::domain::FontFamily>,
            _font_size: Option<crate::shared::config::domain::FontSize>,
        ) -> Size {
            Size::new(10, 10)
        }
    }

    struct MockAnyModulePort {
        render_node: crate::features::layout_engine::domain::LayoutNode,
        subs: Vec<crate::shared::events::signals::SignalKind>,
    }

    impl AnyModulePort for MockAnyModulePort {
        fn init(
            &mut self,
            _config: &crate::shared::config::domain::ModuleConfig,
            _full_config: &crate::shared::config::domain::Config,
        ) -> Result<(), crate::features::module_runtime::ports::ModuleInitError> {
            Ok(())
        }

        fn subscriptions(&self) -> &[crate::shared::events::signals::SignalKind] {
            &self.subs
        }

        fn styles(&self) -> &[crate::features::styling::domain::StyleSheetName] {
            &[]
        }

        fn refresh(
            &mut self,
            _hub: &SignalHub,
            _signals: &[crate::shared::events::signals::SignalKind],
        ) {
        }

        fn render(
            &self,
            _monitor: &MonitorId,
        ) -> crate::features::layout_engine::domain::LayoutNode {
            self.render_node.clone()
        }

        fn call_function(
            &mut self,
            _name: &crate::shared::primitives::FunctionName,
        ) -> Result<(), crate::features::module_runtime::ports::ModuleInitError> {
            Ok(())
        }
    }

    #[test]
    fn test_module_context_accessors() {
        let id = crate::shared::primitives::ModuleId::new(1);
        let hub = Arc::new(SignalHub::new(Config::default()));
        let sm: DynSurfaceManager = Arc::new(MockSurfaceManager);
        let cmd_tx: Arc<dyn CommandSender> = Arc::new(MockCommandSender);
        let (_layout_tx, layout_rx) = watch::channel(HashMap::new());

        let mut ctx = ModuleContext::new(id, hub.clone(), sm.clone(), cmd_tx, layout_rx);
        assert_eq!(ctx.id(), id);
        assert_eq!(Arc::as_ptr(ctx.hub()), Arc::as_ptr(&hub));

        let (rx1, _rx2) = ctx.rxs_mut();
        let _ = rx1.borrow();
    }

    #[tokio::test]
    async fn test_module_actor_measure_and_render_all() {
        use crate::shared::primitives::geometry::Rect;

        let id = crate::shared::primitives::ModuleId::new(1);
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config));

        {
            let mut monitors = std::collections::BTreeMap::new();
            monitors.insert(
                crate::features::workspaces::domain::MonitorName::new("DP-1"),
                crate::features::workspaces::domain::Monitor::new(
                    crate::features::workspaces::domain::MonitorName::new("DP-1"),
                    crate::features::workspaces::domain::WorkspaceId::new(1),
                    None,
                ),
            );
            let h_state = crate::shared::events::signals::HyprlandState::new(
                std::collections::BTreeMap::new(),
                monitors,
                Some(crate::features::workspaces::domain::MonitorName::new(
                    "DP-1",
                )),
            );
            hub.hyprland_tx().send(h_state).unwrap();
        }

        let sm: DynSurfaceManager = Arc::new(MockSurfaceManager);
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();
        let sender: Arc<dyn CommandSender> = Arc::new(TestCommandSender { tx: cmd_tx });

        let (layout_tx, layout_rx) = watch::channel(HashMap::new());
        let ctx = ModuleContext::new(id, hub.clone(), sm.clone(), sender, layout_rx);

        let port = Box::new(MockAnyModulePort {
            render_node: crate::features::layout_engine::domain::LayoutNode::Rect {
                class: None,
                id: None,
                on_click: None,
                on_hover: None,
                tooltip: None,
            },
            subs: vec![
                crate::shared::events::signals::SignalKind::Time,
                crate::shared::events::signals::SignalKind::Hyprland,
                crate::shared::events::signals::SignalKind::Applets,
                crate::shared::events::signals::SignalKind::Metrics,
                crate::shared::events::signals::SignalKind::DBus(
                    crate::shared::dbus::domain::DBusSubscription::new(
                        crate::shared::dbus::domain::BusType::Session,
                        None,
                        None,
                        None,
                        None,
                    ),
                ),
            ],
        });

        let resolver = Arc::new(
            crate::features::styling::adapters::fs_loader::CompositeStyleResolver::new(vec![]),
        );
        let mut actor = ModuleActor::new(
            port,
            ctx,
            Arc::new(std::sync::Mutex::new(MockCanvasFactory)),
            resolver,
        );

        let mut actor = tokio::task::spawn_blocking(move || {
            let mut layout_engines = HashMap::new();
            actor.measure_and_render_all(&mut layout_engines);
            actor
        })
        .await
        .unwrap();

        let cmd = cmd_rx.try_recv().expect("Should send size changed command");
        match cmd {
            AppCommand::ModuleSizeChanged(mon, mod_id, size) => {
                assert_eq!(mon.as_str(), "DP-1");
                assert_eq!(mod_id, id);
                assert_eq!(size.width(), 10);
                assert_eq!(size.height(), 10);
            }
            _ => panic!("Unexpected command"),
        }

        let mut layouts = HashMap::new();
        layouts.insert(
            MonitorId::new("DP-1"),
            Rect::new(
                crate::shared::primitives::geometry::Position::new(0, 0),
                crate::shared::primitives::geometry::Size::new(10, 10),
            ),
        );
        layout_tx.send(layouts).unwrap();

        let _ = tokio::task::spawn_blocking(move || {
            let mut layout_engines = HashMap::new();
            actor.measure_and_render_all(&mut layout_engines);
            actor
        })
        .await
        .unwrap();
        assert!(cmd_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_module_actor_lifecycle() {
        let id = crate::shared::primitives::ModuleId::new(2);
        let hub = Arc::new(SignalHub::new(Config::default()));

        let sm: DynSurfaceManager = Arc::new(MockSurfaceManager);
        let sender: Arc<dyn CommandSender> = Arc::new(MockCommandSender);
        let (layout_tx, layout_rx) = watch::channel(HashMap::new());
        let ctx = ModuleContext::new(id, hub.clone(), sm, sender, layout_rx);

        let click_node = crate::features::layout_engine::domain::LayoutNode::Rect {
            class: None,
            id: None,
            on_click: Some(crate::app::commands::AppCommand::RequestRender),
            on_hover: Some(crate::app::commands::AppCommand::RequestRender),
            tooltip: None,
        };

        let port = Box::new(MockAnyModulePort {
            render_node: click_node,
            subs: vec![
                crate::shared::events::signals::SignalKind::Time,
                crate::shared::events::signals::SignalKind::Hyprland,
                crate::shared::events::signals::SignalKind::Applets,
                crate::shared::events::signals::SignalKind::Metrics,
                crate::shared::events::signals::SignalKind::DBus(
                    crate::shared::dbus::domain::DBusSubscription::new(
                        crate::shared::dbus::domain::BusType::Session,
                        None,
                        None,
                        None,
                        None,
                    ),
                ),
            ],
        });

        let resolver = Arc::new(
            crate::features::styling::adapters::fs_loader::CompositeStyleResolver::new(vec![]),
        );
        let mut actor = ModuleActor::new(
            port,
            ctx,
            Arc::new(std::sync::Mutex::new(MockCanvasFactory)),
            resolver,
        );
        // Pre-populate render_trees for hit testing
        actor.render_trees.insert(
            MonitorId::new("DP-1"),
            crate::features::layout_engine::domain::RenderNode::Rect {
                rect: crate::shared::primitives::geometry::Rect::new(
                    crate::shared::primitives::geometry::Position::new(0, 0),
                    crate::shared::primitives::geometry::Size::new(100, 100),
                ),
                style: crate::features::styling::domain::ComputedStyle::default(),
                on_click: Some(AppCommand::RequestRender),
                on_hover: Some(AppCommand::RequestRender),
                tooltip: None,
            },
        );

        actor.spawn();

        hub.time_tx().send(chrono::Local::now()).unwrap();
        hub.applets_tx()
            .send(crate::features::applets::domain::AppletsState::default())
            .unwrap();
        hub.hyprland_tx()
            .send(crate::shared::events::signals::HyprlandState::new(
                std::collections::BTreeMap::new(),
                std::collections::BTreeMap::new(),
                None,
            ))
            .unwrap();

        let _ = hub.pointer_tx().send((
            id,
            MonitorId::new("DP-1"),
            crate::shared::events::core::PointerEvent::Click {
                x: 50.0,
                y: 50.0,
                button: 1,
            },
        ));
        let _ = hub.pointer_tx().send((
            id,
            MonitorId::new("DP-1"),
            crate::shared::events::core::PointerEvent::PointerMotion { x: 50.0, y: 50.0 },
        ));

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        drop(layout_tx);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    #[tokio::test]
    async fn test_module_actor_renders_unknown_hyprland_monitor_from_layout() {
        use crate::shared::primitives::geometry::Rect;

        let id = crate::shared::primitives::ModuleId::new(1);
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config));

        // 1. Hyprland state is empty (no monitors)
        let h_state = crate::shared::events::signals::HyprlandState::new(
            std::collections::BTreeMap::new(),
            std::collections::BTreeMap::new(),
            None,
        );
        hub.hyprland_tx().send(h_state).unwrap();

        let sm: DynSurfaceManager = Arc::new(MockSurfaceManager);
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();
        let sender: Arc<dyn CommandSender> = Arc::new(TestCommandSender { tx: cmd_tx });

        let (layout_tx, layout_rx) = watch::channel(HashMap::new());
        let ctx = ModuleContext::new(id, hub.clone(), sm.clone(), sender, layout_rx);

        let port = Box::new(MockAnyModulePort {
            render_node: crate::features::layout_engine::domain::LayoutNode::Rect {
                class: None,
                id: None,
                on_click: None,
                on_hover: None,
                tooltip: None,
            },
            subs: vec![
                crate::shared::events::signals::SignalKind::Time,
                crate::shared::events::signals::SignalKind::Hyprland,
                crate::shared::events::signals::SignalKind::Applets,
                crate::shared::events::signals::SignalKind::Metrics,
                crate::shared::events::signals::SignalKind::DBus(
                    crate::shared::dbus::domain::DBusSubscription::new(
                        crate::shared::dbus::domain::BusType::Session,
                        None,
                        None,
                        None,
                        None,
                    ),
                ),
            ],
        });

        let resolver = Arc::new(
            crate::features::styling::adapters::fs_loader::CompositeStyleResolver::new(vec![]),
        );
        let mut actor = ModuleActor::new(
            port,
            ctx,
            Arc::new(std::sync::Mutex::new(MockCanvasFactory)),
            resolver,
        );

        // 2. Wayland sends layout update for DP-2
        let mut layouts = HashMap::new();
        layouts.insert(
            MonitorId::new("DP-2"),
            Rect::new(
                crate::shared::primitives::geometry::Position::new(0, 0),
                crate::shared::primitives::geometry::Size::new(10, 10),
            ),
        );
        layout_tx.send(layouts).unwrap();

        let _ = tokio::task::spawn_blocking(move || {
            let mut layout_engines = HashMap::new();
            // 3. This should process DP-2 because it's in the layouts map, even though hyprland state is empty!
            actor.measure_and_render_all(&mut layout_engines);
            actor
        })
        .await
        .unwrap();

        // 4. Verify the actor processed the monitor and emitted a size changed command
        let cmd = cmd_rx
            .try_recv()
            .expect("Should send size changed command for DP-2");
        match cmd {
            AppCommand::ModuleSizeChanged(mon, mod_id, size) => {
                assert_eq!(mon.as_str(), "DP-2");
                assert_eq!(mod_id, id);
                assert_eq!(size.width(), 10);
            }
            _ => panic!("Unexpected command"),
        }
    }
}
