use crate::domain::commands::AppCommand;
use crate::domain::shared::render::RenderBuffer;
use crate::domain::signals::SignalHub;
use crate::domain::{
    MonitorId,
    shared::geometry::{Rect, Scale, Size},
};
use crate::ports::registry::AnyModulePort;
use crate::ports::surface::DynSurfaceManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::watch;

pub struct ModuleContext {
    id: crate::domain::ModuleId,
    hub: Arc<SignalHub>,
    surface_manager: DynSurfaceManager,
    command_tx: Arc<dyn crate::ports::registry::CommandSender>,
    layout_rx: watch::Receiver<HashMap<MonitorId, Rect>>,
    pointer_rx: crate::domain::events::PointerReceiver,
}

impl ModuleContext {
    pub fn new(
        id: crate::domain::ModuleId,
        hub: Arc<SignalHub>,
        surface_manager: DynSurfaceManager,
        command_tx: Arc<dyn crate::ports::registry::CommandSender>,
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

    pub fn id(&self) -> crate::domain::ModuleId {
        self.id
    }

    pub fn hub(&self) -> &Arc<SignalHub> {
        &self.hub
    }

    pub fn surface_manager(&self) -> &DynSurfaceManager {
        &self.surface_manager
    }

    pub fn command_tx(&self) -> &dyn crate::ports::registry::CommandSender {
        self.command_tx.as_ref()
    }

    pub fn rxs_mut(
        &mut self,
    ) -> (
        &mut watch::Receiver<HashMap<MonitorId, Rect>>,
        &mut crate::domain::events::PointerReceiver,
    ) {
        (&mut self.layout_rx, &mut self.pointer_rx)
    }
}

pub struct ModuleActor<F: crate::ports::canvas::CanvasFactory + 'static> {
    port: Box<dyn AnyModulePort>,
    ctx: ModuleContext,
    sizes: std::collections::HashMap<MonitorId, Size>,
    canvas_factory: std::sync::Arc<std::sync::Mutex<F>>,
    render_trees: std::collections::HashMap<MonitorId, crate::domain::layout::RenderNode>,
}

impl<F: crate::ports::canvas::CanvasFactory + 'static> ModuleActor<F> {
    pub fn new(
        port: Box<dyn AnyModulePort>,
        ctx: ModuleContext,
        canvas_factory: std::sync::Arc<std::sync::Mutex<F>>,
    ) -> Self {
        Self {
            port,
            ctx,
            sizes: std::collections::HashMap::new(),
            canvas_factory,
            render_trees: std::collections::HashMap::new(),
        }
    }

    pub fn spawn(mut self) {
        tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Handle::current();

            use futures_util::stream::{SelectAll, StreamExt};
            use tokio_stream::wrappers::WatchStream;
            use crate::domain::signals::SignalKind;

            let mut events_stream = SelectAll::new();
            let subs = self.port.subscriptions();

            if subs.contains(&SignalKind::Time) {
                events_stream.push(WatchStream::new(self.ctx.hub().time_rx()).map(|_| SignalKind::Time).boxed());
            }
            if subs.contains(&SignalKind::Hyprland) {
                events_stream.push(WatchStream::new(self.ctx.hub().hyprland_rx()).map(|_| SignalKind::Hyprland).boxed());
            }
            if subs.contains(&SignalKind::Applets) {
                events_stream.push(WatchStream::new(self.ctx.hub().applets_rx()).map(|_| SignalKind::Applets).boxed());
            }
            if subs.contains(&SignalKind::Metrics) {
                events_stream.push(WatchStream::new(self.ctx.hub().metrics_rx()).map(|_| SignalKind::Metrics).boxed());
            }
            if subs.iter().any(|s| matches!(s, SignalKind::DBus(_))) {
                events_stream.push(WatchStream::new(self.ctx.hub().dbus_rx()).map(|_| SignalKind::DBus(crate::domain::dbus::DBusSubscription { bus: crate::domain::dbus::BusType::Session, destination: None, path: None, interface: None, member: None })).boxed());
            }

            let mut layout_engines: std::collections::HashMap<MonitorId, Box<dyn crate::ports::layout::LayoutEnginePort>> = std::collections::HashMap::new();

            // Initial refresh
            let initial_sigs = self.port.subscriptions();
            self.port.refresh(self.ctx.hub(), &initial_sigs);
            self.measure_and_render_all(&mut layout_engines);

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
                        Ok((target_id, monitor_id, event)) = input_rx.recv() => {
                            if target_id == ctx_id {
                                tracing::debug!(module = %ctx_id, monitor = %monitor_id, event = ?event, "Received pointer event in module actor");
                                if let Some(render_tree) = self.render_trees.get(&monitor_id) {
                                    use crate::domain::shared::geometry::Position;
                                    use crate::domain::events::PointerEvent;
                                    match event {
                                        PointerEvent::Click { x, y, .. } => {
                                            let pos = Position::new(x as i32, y as i32);
                                            let hit = render_tree.hit_test(pos);
                                            tracing::debug!(module = %ctx_id, pos = ?pos, hit = ?hit.is_some(), "Hit test for click");
                                            if let Some(hit) = hit
                                                && let Some(cmd) = hit.on_click() {
                                                    tracing::debug!(module = %ctx_id, cmd = ?cmd, "Sending on_click command");
                                                    self.ctx.command_tx().send_command(cmd.clone());
                                                }
                                        },
                                        PointerEvent::PointerMotion { x, y } => {
                                            let pos = Position::new(x as i32, y as i32);
                                            let hit = render_tree.hit_test(pos);
                                            if let Some(hit) = hit
                                                && let Some(cmd) = hit.on_hover() {
                                                    tracing::debug!(module = %ctx_id, cmd = ?cmd, "Sending on_hover command");
                                                    self.ctx.command_tx().send_command(cmd.clone());
                                                }
                                        },
                                        _ => {}
                                    }
                                } else {
                                    tracing::warn!(module = %ctx_id, monitor = %monitor_id, "Received pointer event but no render tree found for monitor");
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
            }
        });
    }

    #[tracing::instrument(level = "debug", skip(self, layout_engines), fields(module = %self.ctx.id()))]
    fn measure_and_render_all(
        &mut self,
        layout_engines: &mut std::collections::HashMap<MonitorId, Box<dyn crate::ports::layout::LayoutEnginePort>>
    ) {
        let t0 = std::time::Instant::now();
        let monitors: Vec<MonitorId> = self
            .ctx
            .hub()
            .hyprland_rx()
            .borrow()
            .monitors()
            .values()
            .map(|m| MonitorId::new(m.name().as_str()))
            .collect();
        let layouts: std::collections::HashMap<MonitorId, Rect> =
            self.ctx.rxs_mut().0.borrow().clone();

        for monitor_id in monitors {
            let layout_node = self.port.render(&monitor_id);

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
                    Box::new(crate::adapters::taffy_layout::TaffyLayoutAdapter::new())
                });
                
                engine.calculate_layout(layout_node, &mut measurer, crate::domain::shared::geometry::Position::new(0, 0))
            };
            
            let render_node = match render_node_res {
                Ok(node) => node,
                Err(e) => {
                    eprintln!("Module layout failed: {}", e);
                    continue;
                }
            };

            self.render_trees.insert(monitor_id.clone(), render_node.clone());

            let size = *render_node.rect().size();

            let old_size = self
                .sizes
                .get(&monitor_id)
                .copied()
                .unwrap_or(Size::new(0, 0));
            if size != old_size {
                self.sizes.insert(monitor_id.clone(), size);
                self
                    .ctx
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
                let position = crate::domain::shared::geometry::Position::new(bounds.x(), bounds.y());
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
    use crate::domain::config::Config;
    use crate::ports::registry::CommandSender;
    
    struct MockCommandSender;
    impl CommandSender for MockCommandSender {
        fn send_command(&self, _cmd: AppCommand) {}
    }

    struct MockSurfaceManager;
    #[async_trait::async_trait]
    impl crate::ports::surface::SurfaceManagerPort for MockSurfaceManager {
        async fn submit_buffer(&self, _mod_id: crate::domain::ModuleId, _mon_id: MonitorId, _pos: crate::domain::shared::geometry::Position, _buf: RenderBuffer) {}
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
    
    impl crate::ports::canvas::CanvasFactory for MockCanvasFactory {
        fn create_canvas<'a>(
            &'a mut self,
            _data: &'a mut [u8],
            _size: Size,
            _scale: Scale,
            _font_family: crate::domain::config::FontFamily,
            _font_size: crate::domain::config::FontSize,
        ) -> impl crate::ports::canvas::Canvas + 'a {
            MockCanvas
        }
        
        fn create_text_measurer<'a>(
            &'a mut self,
            _scale: Scale,
            _font_family: crate::domain::config::FontFamily,
            _font_size: crate::domain::config::FontSize,
        ) -> impl crate::domain::layout::TextMeasurer + 'a {
            MockMeasurer
        }
    }
    
    struct MockCanvas;
    impl crate::ports::canvas::Canvas for MockCanvas {
        fn draw_rect(&mut self, _x: crate::domain::shared::geometry::LogicalPx, _y: crate::domain::shared::geometry::LogicalPx, _w: crate::domain::shared::geometry::LogicalPx, _h: crate::domain::shared::geometry::LogicalPx, _color: crate::domain::shared::color::DrawingColor, _radius: crate::domain::shared::geometry::LogicalPx) {}
        fn draw_border(&mut self, _pos: crate::domain::shared::geometry::Position, _size: crate::domain::shared::geometry::Size, _color: crate::domain::shared::color::DrawingColor, _radius: crate::domain::shared::geometry::LogicalPx, _border_size: crate::domain::shared::geometry::LogicalPx) {}
        fn draw_text(&mut self, _text: &str, _font_family: Option<&crate::domain::config::FontFamily>, _font_size: Option<crate::domain::config::FontSize>, _color: crate::domain::shared::color::DrawingColor, _pos: crate::domain::shared::geometry::Position) {}
        fn draw_image(&mut self, _image_data: &[u8], _pixel_size: crate::domain::shared::geometry::Size, _logical_size: crate::domain::shared::geometry::Size, _pos: crate::domain::shared::geometry::Position) {}
    }
    
    struct MockMeasurer;
    impl crate::domain::layout::TextMeasurer for MockMeasurer {
        fn measure(&mut self, _text: &str, _font_family: Option<&crate::domain::config::FontFamily>, _font_size: Option<crate::domain::config::FontSize>) -> Size {
            Size::new(10, 10)
        }
    }

    struct MockAnyModulePort {
        render_node: crate::domain::layout::LayoutNode,
    }
    
    impl AnyModulePort for MockAnyModulePort {
        fn init(&mut self, _config: &crate::domain::config::ModuleConfig, _bar_config: &crate::domain::config::BarConfig) -> Result<(), String> {
            Ok(())
        }
        
        fn subscriptions(&self) -> Vec<crate::domain::signals::SignalKind> {
            vec![crate::domain::signals::SignalKind::Time, crate::domain::signals::SignalKind::Hyprland, crate::domain::signals::SignalKind::Applets, crate::domain::signals::SignalKind::Metrics, crate::domain::signals::SignalKind::DBus(crate::domain::dbus::DBusSubscription { bus: crate::domain::dbus::BusType::Session, destination: None, path: None, interface: None, member: None })]
        }
        
        fn refresh(&mut self, _hub: &SignalHub, _signals: &[crate::domain::signals::SignalKind]) {
        }
        
        fn render(&self, _monitor: &MonitorId) -> crate::domain::layout::LayoutNode {
            self.render_node.clone()
        }
    }

    #[test]
    fn test_module_context_accessors() {
        let id = crate::domain::ModuleId::new(1);
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
        use crate::domain::shared::geometry::Rect;
        
        let id = crate::domain::ModuleId::new(1);
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config));
        
        {
            let mut monitors = std::collections::BTreeMap::new();
            monitors.insert(
                crate::domain::workspace::MonitorName::new("DP-1"),
                crate::domain::workspace::Monitor::new(
                    crate::domain::workspace::MonitorName::new("DP-1"),
                    crate::domain::workspace::WorkspaceId::new(1),
                    None,
                ),
            );
            let mut h_state = crate::domain::signals::HyprlandState::new(
                std::collections::BTreeMap::new(),
                monitors,
                Some(crate::domain::workspace::MonitorName::new("DP-1"))
            );
            h_state.apply_event(&crate::domain::events::WindowManagerEvent::MonitorFocused {
                monitor_name: crate::domain::workspace::MonitorName::new("DP-1"),
                workspace_id: crate::domain::workspace::WorkspaceId::new(1),
            });
            hub.hyprland_tx().send(h_state).unwrap();
        }

        let sm: DynSurfaceManager = Arc::new(MockSurfaceManager);
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();
        let sender: Arc<dyn CommandSender> = Arc::new(TestCommandSender { tx: cmd_tx });
        
        let (layout_tx, layout_rx) = watch::channel(HashMap::new());
        let ctx = ModuleContext::new(id, hub.clone(), sm.clone(), sender, layout_rx);
        
        let port = Box::new(MockAnyModulePort {
            render_node: crate::domain::layout::LayoutNode::Rect {
                size: crate::domain::shared::geometry::Size::new(100, 20),
                color: crate::domain::shared::color::DrawingColor::parse("#000000").unwrap(),
                radius: None,
                on_click: None,
                on_hover: None,
            },
        });
        
        let mut actor = ModuleActor::new(port, ctx, Arc::new(std::sync::Mutex::new(MockCanvasFactory)));
        
        let mut actor = tokio::task::spawn_blocking(move || {
            let mut layout_engines = HashMap::new();
            actor.measure_and_render_all(&mut layout_engines);
            actor
        }).await.unwrap();
        
        let cmd = cmd_rx.try_recv().expect("Should send size changed command");
        match cmd {
            AppCommand::ModuleSizeChanged(mon, mod_id, size) => {
                assert_eq!(mon.as_str(), "DP-1");
                assert_eq!(mod_id, id);
                assert_eq!(size.width(), 100);
                assert_eq!(size.height(), 20);
            }
            _ => panic!("Unexpected command"),
        }
        
        let mut layouts = HashMap::new();
        layouts.insert(MonitorId::new("DP-1"), Rect::new(crate::domain::shared::geometry::Position::new(0, 0), crate::domain::shared::geometry::Size::new(100, 20)));
        layout_tx.send(layouts).unwrap();
        
        let _ = tokio::task::spawn_blocking(move || {
            let mut layout_engines = HashMap::new();
            actor.measure_and_render_all(&mut layout_engines);
            actor
        }).await.unwrap();
        assert!(cmd_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_module_actor_lifecycle() {
        let id = crate::domain::ModuleId::new(2);
        let hub = Arc::new(SignalHub::new(Config::default()));
        
        let sm: DynSurfaceManager = Arc::new(MockSurfaceManager);
        let sender: Arc<dyn CommandSender> = Arc::new(MockCommandSender);
        let (layout_tx, layout_rx) = watch::channel(HashMap::new());
        let ctx = ModuleContext::new(id, hub.clone(), sm, sender, layout_rx);
        
        let click_node = crate::domain::layout::LayoutNode::Rect {
            size: crate::domain::shared::geometry::Size::new(100, 100),
            color: crate::domain::shared::color::DrawingColor::parse("#000000").unwrap(),
            radius: None,
            on_click: Some(AppCommand::RequestRender),
            on_hover: Some(AppCommand::RequestRender),
        };

        let port = Box::new(MockAnyModulePort {
            render_node: click_node,
        });
        
        let mut actor = ModuleActor::new(port, ctx, Arc::new(std::sync::Mutex::new(MockCanvasFactory)));
        // Pre-populate render_trees for hit testing
        actor.render_trees.insert(MonitorId::new("DP-1"), crate::domain::layout::RenderNode::Rect {
            rect: crate::domain::shared::geometry::Rect::new(crate::domain::shared::geometry::Position::new(0, 0), crate::domain::shared::geometry::Size::new(100, 100)),
            color: crate::domain::shared::color::DrawingColor::parse("#000000").unwrap(),
            radius: None,
            on_click: Some(AppCommand::RequestRender),
            on_hover: Some(AppCommand::RequestRender),
        });
        
        actor.spawn();
        
        hub.time_tx().send(chrono::Local::now()).unwrap();
        hub.applets_tx().send(crate::domain::applets::AppletsState::default()).unwrap();
        hub.hyprland_tx().send(crate::domain::signals::HyprlandState::new(std::collections::BTreeMap::new(), std::collections::BTreeMap::new(), None)).unwrap();
        
        let _ = hub.pointer_tx().send((id, MonitorId::new("DP-1"), crate::domain::events::PointerEvent::Click { x: 50.0, y: 50.0, button: 1 }));
        let _ = hub.pointer_tx().send((id, MonitorId::new("DP-1"), crate::domain::events::PointerEvent::PointerMotion { x: 50.0, y: 50.0 }));

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        
        drop(layout_tx);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}
