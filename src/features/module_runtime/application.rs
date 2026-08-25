use crate::features::module_runtime::adapters::EventLoop;
use crate::features::module_runtime::domain::{
    ModuleIdentity, PointerHandler, RenderPipeline,
};
use crate::features::module_runtime::ports::{AnyModulePort, CommandSender};
use crate::features::styling::ports::StyleResolverPort;
use crate::features::vdom::ports::VdomDiffPort;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::{
    ModuleId, ModuleInstanceId, MonitorId, geometry::Rect,
};
use crate::shared::rendering::ports::canvas::CanvasFactory;
use crate::shared::wayland::ports::DynSurfaceManager;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::watch;

pub struct ModuleContext {
    identity: ModuleIdentity,
    hub: Arc<SignalHub>,
    surface_manager: DynSurfaceManager,
    command_tx: Arc<dyn CommandSender>,
    layout_rx: watch::Receiver<HashMap<MonitorId, Rect>>,
    pointer_rx: crate::shared::events::core::PointerReceiver,
}

impl ModuleContext {
    pub fn new(
        id: ModuleId,
        hub: Arc<SignalHub>,
        surface_manager: DynSurfaceManager,
        command_tx: Arc<dyn CommandSender>,
        layout_rx: watch::Receiver<HashMap<MonitorId, Rect>>,
    ) -> Self {
        let pointer_rx = hub.pointer_rx();
        Self {
            identity: ModuleIdentity::new(id),
            hub,
            surface_manager,
            command_tx,
            layout_rx,
            pointer_rx,
        }
    }

    pub fn with_parent(mut self, parent_id: Option<ModuleId>) -> Self {
        self.identity = self.identity.with_parent(parent_id);
        self
    }

    pub fn with_instance_id(mut self, instance_id: Option<ModuleInstanceId>) -> Self {
        self.identity = self.identity.with_instance_id(instance_id);
        self
    }

    pub fn identity(&self) -> &ModuleIdentity {
        &self.identity
    }

    pub fn id(&self) -> ModuleId {
        self.identity.id()
    }

    pub fn parent_id(&self) -> Option<ModuleId> {
        self.identity.parent_id()
    }

    pub fn instance_id(&self) -> Option<&ModuleInstanceId> {
        self.identity.instance_id()
    }

    pub fn hub(&self) -> &Arc<SignalHub> {
        &self.hub
    }

    pub fn surface_manager(&self) -> &DynSurfaceManager {
        &self.surface_manager
    }

    pub fn command_tx(&self) -> &dyn CommandSender {
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

pub struct ModuleActor<F: CanvasFactory + 'static> {
    port: Box<dyn AnyModulePort>,
    ctx: ModuleContext,
    canvas_factory: Arc<Mutex<F>>,
    style_resolver: Arc<dyn StyleResolverPort>,
    vdom_diff: Arc<dyn VdomDiffPort>,
}

impl<F: CanvasFactory + 'static> ModuleActor<F> {
    pub fn new(
        port: Box<dyn AnyModulePort>,
        ctx: ModuleContext,
        canvas_factory: Arc<Mutex<F>>,
        style_resolver: Arc<dyn StyleResolverPort>,
        vdom_diff: Arc<dyn VdomDiffPort>,
    ) -> Self {
        Self {
            port,
            ctx,
            canvas_factory,
            style_resolver,
            vdom_diff,
        }
    }

    pub fn spawn(self) {
        let event_loop = EventLoop::new(
            self.port,
            self.ctx,
            PointerHandler::new(),
            RenderPipeline::new(),
            self.canvas_factory,
            self.style_resolver,
            self.vdom_diff,
        );

        tokio::task::spawn_blocking(move || {
            event_loop.run();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::commands::AppCommand;
    use crate::features::module_runtime::test_support::{
        ChannelCommandSender, MockCanvasFactory, MockCommandSender, MockSurfaceManager,
        TestModulePort,
    };
    use crate::features::styling::adapters::fs_loader::CompositeStyleResolver;
    use crate::features::vdom::adapters::DefaultVdomDiffAdapter;
    use crate::features::vdom::domain::VNode;
    use crate::shared::config::domain::Config;
    use crate::shared::events::signals::{HyprlandState, SignalKind};
    use crate::shared::primitives::geometry::{Position, Size};

    struct TestFixture {
        pub hub: Arc<SignalHub>,
        pub cmd_rx: std::sync::mpsc::Receiver<AppCommand>,
        pub layout_tx: watch::Sender<HashMap<MonitorId, Rect>>,
        pub event_loop: EventLoop<MockCanvasFactory>,
    }

    struct TestFixtureBuilder {
        id: ModuleId,
        monitors: Vec<&'static str>,
        subs: Vec<SignalKind>,
        vnode: VNode,
    }

    impl TestFixtureBuilder {
        fn new(id: ModuleId) -> Self {
            Self {
                id,
                monitors: Vec::new(),
                subs: Vec::new(),
                vnode: VNode::new_rect(None, None, None, None, None),
            }
        }

        fn with_monitors(mut self, monitors: &[&'static str]) -> Self {
            self.monitors = monitors.to_vec();
            self
        }

        fn with_subs(mut self, subs: Vec<SignalKind>) -> Self {
            self.subs = subs;
            self
        }

        fn with_vnode(mut self, vnode: VNode) -> Self {
            self.vnode = vnode;
            self
        }

        fn build(self) -> TestFixture {
            let hub = Arc::new(SignalHub::new(Config::default()));

            if !self.monitors.is_empty() {
                let mut monitors_map = std::collections::BTreeMap::new();
                for m in &self.monitors {
                    let name = crate::features::workspaces::domain::MonitorName::new(*m);
                    monitors_map.insert(
                        name.clone(),
                        crate::features::workspaces::domain::Monitor::new(
                            name,
                            crate::features::workspaces::domain::WorkspaceId::new(1),
                            None,
                        ),
                    );
                }
                let focused = self
                    .monitors
                    .first()
                    .map(|m| crate::features::workspaces::domain::MonitorName::new(*m));
                let h_state = HyprlandState::new(
                    std::collections::BTreeMap::new(),
                    monitors_map,
                    focused,
                );
                hub.hyprland_tx().send(h_state).unwrap();
            }

            let sm: DynSurfaceManager = Arc::new(MockSurfaceManager);
            let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();
            let sender: Arc<dyn CommandSender> = Arc::new(ChannelCommandSender { tx: cmd_tx });
            let (layout_tx, layout_rx) = watch::channel(HashMap::new());

            let ctx = ModuleContext::new(self.id, hub.clone(), sm, sender, layout_rx);
            let port = Box::new(TestModulePort::with_subs(self.vnode, self.subs));

            let resolver = Arc::new(CompositeStyleResolver::new(vec![]));
            let diff_adapter = Arc::new(DefaultVdomDiffAdapter::new());
            let canvas_factory = Arc::new(Mutex::new(MockCanvasFactory));

            let event_loop = EventLoop::new(
                port,
                ctx,
                PointerHandler::new(),
                RenderPipeline::new(),
                canvas_factory,
                resolver,
                diff_adapter,
            );

            TestFixture {
                hub,
                cmd_rx,
                layout_tx,
                event_loop,
            }
        }
    }

    #[test]
    fn test_module_context_accessors() {
        let id = ModuleId::new(1);
        let hub = Arc::new(SignalHub::new(Config::default()));
        let sm: DynSurfaceManager = Arc::new(MockSurfaceManager);
        let cmd_tx: Arc<dyn CommandSender> = Arc::new(MockCommandSender);
        let (_layout_tx, layout_rx) = watch::channel(HashMap::new());

        let mut ctx = ModuleContext::new(id, hub.clone(), sm, cmd_tx, layout_rx)
            .with_parent(Some(ModuleId::new(99)))
            .with_instance_id(Some(ModuleInstanceId::new("inst-1")));

        assert_eq!(ctx.id(), id);
        assert_eq!(ctx.parent_id(), Some(ModuleId::new(99)));
        assert_eq!(ctx.instance_id(), Some(&ModuleInstanceId::new("inst-1")));
        assert_eq!(Arc::as_ptr(ctx.hub()), Arc::as_ptr(&hub));

        let (rx1, _rx2) = ctx.rxs_mut();
        let _ = rx1.borrow();
    }

    #[tokio::test]
    async fn test_module_actor_measure_and_render_all() {
        let id = ModuleId::new(1);
        let fixture = TestFixtureBuilder::new(id)
            .with_monitors(&["DP-1"])
            .with_subs(vec![
                SignalKind::Time,
                SignalKind::Hyprland,
                SignalKind::Systray,
                SignalKind::Metrics,
            ])
            .build();

        let mut event_loop = fixture.event_loop;
        let mut event_loop = tokio::task::spawn_blocking(move || {
            let mut layout_engines = HashMap::new();
            event_loop.render_all_monitors(&mut layout_engines);
            event_loop
        })
        .await
        .unwrap();

        let cmd = fixture.cmd_rx.try_recv().expect("Should send size changed command");
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
            Rect::new(Position::new(0, 0), Size::new(10, 10)),
        );
        fixture.layout_tx.send(layouts).unwrap();

        let _ = tokio::task::spawn_blocking(move || {
            let mut layout_engines = HashMap::new();
            event_loop.render_all_monitors(&mut layout_engines);
            event_loop
        })
        .await
        .unwrap();
        assert!(fixture.cmd_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_module_actor_lifecycle() {
        let id = ModuleId::new(2);
        let click_node = VNode::new_rect(
            None,
            None,
            Some(AppCommand::RequestRender),
            Some(AppCommand::RequestRender),
            None,
        );

        let fixture = TestFixtureBuilder::new(id)
            .with_monitors(&["DP-1"])
            .with_subs(vec![
                SignalKind::Time,
                SignalKind::Hyprland,
                SignalKind::Systray,
                SignalKind::Metrics,
            ])
            .with_vnode(click_node)
            .build();

        let actor = fixture.event_loop.into_actor();
        actor.spawn();

        fixture.hub.time_tx().send(chrono::Local::now()).unwrap();
        fixture
            .hub
            .systray_tx()
            .send(crate::features::systray::domain::SystrayState::default())
            .unwrap();

        let _ = fixture.hub.pointer_tx().send((
            id,
            MonitorId::new("DP-1"),
            crate::shared::events::core::PointerEvent::Click {
                x: 5.0,
                y: 5.0,
                button: 1,
            },
        ));

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        drop(fixture.layout_tx);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn test_module_actor_renders_unknown_hyprland_monitor_from_layout() {
        let id = ModuleId::new(1);
        let fixture = TestFixtureBuilder::new(id).build();

        let mut layouts = HashMap::new();
        layouts.insert(
            MonitorId::new("DP-2"),
            Rect::new(Position::new(0, 0), Size::new(10, 10)),
        );
        fixture.layout_tx.send(layouts).unwrap();

        let mut event_loop = fixture.event_loop;
        let _ = tokio::task::spawn_blocking(move || {
            let mut layout_engines = HashMap::new();
            event_loop.render_all_monitors(&mut layout_engines);
            event_loop
        })
        .await
        .unwrap();

        let cmd = fixture
            .cmd_rx
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

    #[tokio::test]
    async fn test_module_actor_skips_pipeline_when_vdom_unchanged() {
        let id = ModuleId::new(1);
        let node = VNode::new_text(
            crate::features::vdom::domain::TextContent::new("unchanged".to_string()),
            None,
            None,
            None,
            None,
            None,
        );

        let fixture = TestFixtureBuilder::new(id)
            .with_monitors(&["DP-1"])
            .with_vnode(node)
            .build();

        let mut event_loop = fixture.event_loop;
        let mut event_loop = tokio::task::spawn_blocking(move || {
            let mut layout_engines = HashMap::new();
            event_loop.render_all_monitors(&mut layout_engines);
            event_loop
        })
        .await
        .unwrap();

        assert!(event_loop.render_pipeline().vdom_trees().contains_key(&MonitorId::new("DP-1")));
        assert!(event_loop.render_pipeline().render_trees().contains_key(&MonitorId::new("DP-1")));

        // Second run with identical VDOM triggers early continue
        let event_loop = tokio::task::spawn_blocking(move || {
            let mut layout_engines = HashMap::new();
            event_loop.render_all_monitors(&mut layout_engines);
            event_loop
        })
        .await
        .unwrap();

        assert!(event_loop.render_pipeline().vdom_trees().contains_key(&MonitorId::new("DP-1")));
        assert!(event_loop.render_pipeline().render_trees().contains_key(&MonitorId::new("DP-1")));
    }

    #[tokio::test]
    async fn test_container_module_emits_container_layouts_calculated() {
        let id = ModuleId::new(0);
        let child_node = VNode::new_module(
            crate::shared::primitives::ModuleName::new("hour"),
            None,
            crate::shared::primitives::ModuleOptions::default(),
            None,
            None,
            None,
            None,
            None,
        );
        let root_node = VNode::new_flex(vec![child_node], None, None, None, None, None);

        let fixture = TestFixtureBuilder::new(id)
            .with_monitors(&["DP-1"])
            .with_vnode(root_node)
            .build();

        // Report child module size in hub
        let mut sizes_map = HashMap::new();
        let mut mon_map = crate::shared::primitives::ChildSizesMap::new();
        mon_map.insert(
            crate::shared::primitives::ModuleKey::from_name(
                crate::shared::primitives::ModuleName::new("hour"),
            ),
            Size::new(80, 24),
        );
        sizes_map.insert(MonitorId::new("DP-1"), mon_map);
        fixture.hub.module_sizes_tx().send(sizes_map).unwrap();

        let mut layouts = HashMap::new();
        layouts.insert(
            MonitorId::new("DP-1"),
            Rect::new(Position::new(0, 0), Size::new(1920, 30)),
        );
        fixture.layout_tx.send(layouts).unwrap();

        let mut event_loop = fixture.event_loop;
        let _ = tokio::task::spawn_blocking(move || {
            let mut layout_engines = HashMap::new();
            event_loop.render_all_monitors(&mut layout_engines);
            event_loop
        })
        .await
        .unwrap();

        let mut found_container_layouts = false;
        while let Ok(cmd) = fixture.cmd_rx.try_recv() {
            if let AppCommand::ContainerLayoutsCalculated {
                parent_id,
                monitor_id,
                layouts,
            } = cmd
            {
                assert_eq!(parent_id, id);
                assert_eq!(monitor_id.as_str(), "DP-1");
                assert_eq!(layouts.len(), 1);
                assert_eq!(layouts[0].key().name().as_str(), "hour");
                assert_eq!(layouts[0].bounds().width(), 80);
                assert_eq!(layouts[0].bounds().height(), 24);
                found_container_layouts = true;
            }
        }
        assert!(found_container_layouts, "Should emit ContainerLayoutsCalculated");
    }
}
