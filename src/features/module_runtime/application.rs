use crate::features::layout_engine::domain::DisplayCommandSender;
use crate::features::module_runtime::adapters::EventLoop;
use crate::features::module_runtime::domain::{ModuleIdentity, PointerHandler, RenderPipeline};
use crate::features::module_runtime::ports::{AnyModulePort, LayoutEventSender};
use crate::features::styling::ports::StyleResolverPort;
use crate::features::vdom::domain::UiCommandSender;
use crate::features::vdom::ports::VdomDiffPort;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::{ModuleId, ModuleInstanceId, MonitorId, geometry::Rect};
use crate::shared::rendering::ports::canvas::CanvasFactory;
use crate::shared::wayland::ports::DynSurfaceManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::watch;

pub struct ModuleContext<
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
> {
    identity: ModuleIdentity,
    hub: Arc<SignalHub>,
    surface_manager: DynSurfaceManager,
    layout_sender: Arc<LS>,
    display_sender: Arc<DS>,
    ui_sender: Arc<US>,
    layout_rx: watch::Receiver<HashMap<MonitorId, Rect>>,
    pointer_rx: crate::shared::events::core::PointerReceiver,
}

impl<
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
> ModuleContext<LS, DS, US>
{
    pub fn new(
        id: ModuleId,
        hub: Arc<SignalHub>,
        surface_manager: DynSurfaceManager,
        layout_sender: Arc<LS>,
        display_sender: Arc<DS>,
        ui_sender: Arc<US>,
        layout_rx: watch::Receiver<HashMap<MonitorId, Rect>>,
    ) -> Self {
        let pointer_rx = hub.pointer_rx();
        Self {
            identity: ModuleIdentity::new(id),
            hub,
            surface_manager,
            layout_sender,
            display_sender,
            ui_sender,
            layout_rx,
            pointer_rx,
        }
    }

    #[must_use]
    pub fn with_parent(mut self, parent_id: Option<ModuleId>) -> Self {
        self.identity = self.identity.with_parent(parent_id);
        self
    }

    #[must_use]
    pub fn with_instance_id(mut self, instance_id: Option<ModuleInstanceId>) -> Self {
        self.identity = self.identity.with_instance_id(instance_id);
        self
    }

    #[must_use]
    pub const fn identity(&self) -> &ModuleIdentity {
        &self.identity
    }

    #[must_use]
    pub const fn id(&self) -> ModuleId {
        self.identity.id()
    }

    #[must_use]
    pub const fn parent_id(&self) -> Option<ModuleId> {
        self.identity.parent_id()
    }

    #[must_use]
    pub const fn instance_id(&self) -> Option<&ModuleInstanceId> {
        self.identity.instance_id()
    }

    #[must_use]
    pub const fn hub(&self) -> &Arc<SignalHub> {
        &self.hub
    }

    #[must_use]
    pub const fn surface_manager(&self) -> &DynSurfaceManager {
        &self.surface_manager
    }

    #[must_use]
    pub fn layout_sender(&self) -> &LS {
        self.layout_sender.as_ref()
    }

    #[must_use]
    pub fn display_sender(&self) -> &DS {
        self.display_sender.as_ref()
    }

    #[must_use]
    pub fn ui_sender(&self) -> &US {
        self.ui_sender.as_ref()
    }

    pub const fn rxs_mut(
        &mut self,
    ) -> (
        &mut watch::Receiver<HashMap<MonitorId, Rect>>,
        &mut crate::shared::events::core::PointerReceiver,
    ) {
        (&mut self.layout_rx, &mut self.pointer_rx)
    }
}

pub struct ModuleActor<
    F: CanvasFactory + 'static,
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
> {
    port: Box<dyn AnyModulePort>,
    ctx: ModuleContext<LS, DS, US>,
    canvas_factory: F,
    style_resolver: Arc<dyn StyleResolverPort>,
    vdom_diff: Arc<dyn VdomDiffPort>,
}

impl<
    F: CanvasFactory + 'static,
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
> ModuleActor<F, LS, DS, US>
{
    #[must_use]
    pub fn new(
        port: Box<dyn AnyModulePort>,
        ctx: ModuleContext<LS, DS, US>,
        canvas_factory: F,
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

        tokio::spawn(async move {
            event_loop.run().await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::layout_engine::domain::DisplayCommand;
    use crate::features::module_runtime::ports::LayoutEvent;
    use crate::features::module_runtime::test_support::{
        ChannelDisplaySender, ChannelLayoutSender, ChannelUiSender, MockCanvasFactory,
        MockDisplaySender, MockLayoutSender, MockSurfaceManager, MockUiSender, TestModulePort,
    };
    use crate::features::styling::adapters::fs_loader::CompositeStyleResolver;
    use crate::features::vdom::adapters::DefaultVdomDiffAdapter;
    use crate::features::vdom::domain::{UiCommand, VNode};
    use crate::shared::config::domain::Config;
    use crate::shared::events::signals::{HyprlandState, SignalKind};
    use crate::shared::primitives::geometry::{Position, Size};

    #[allow(dead_code)]
    struct TestFixture {
        pub hub: Arc<SignalHub>,
        pub layout_rx: std::sync::mpsc::Receiver<LayoutEvent>,
        pub display_rx: std::sync::mpsc::Receiver<DisplayCommand>,
        pub ui_rx: std::sync::mpsc::Receiver<UiCommand>,
        pub layout_tx: watch::Sender<HashMap<MonitorId, Rect>>,
        pub event_loop: EventLoop<
            MockCanvasFactory,
            ChannelLayoutSender,
            ChannelDisplaySender,
            ChannelUiSender,
        >,
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
                let h_state =
                    HyprlandState::new(std::collections::BTreeMap::new(), monitors_map, focused);
                hub.hyprland_tx().send(h_state).unwrap();
            }

            let sm: DynSurfaceManager = Arc::new(MockSurfaceManager);
            let (layout_event_tx, layout_event_rx) = std::sync::mpsc::channel();
            let (display_tx, display_rx) = std::sync::mpsc::channel();
            let (ui_tx, ui_rx) = std::sync::mpsc::channel();

            let layout_sender = Arc::new(ChannelLayoutSender {
                tx: layout_event_tx,
            });
            let display_sender = Arc::new(ChannelDisplaySender { tx: display_tx });
            let ui_sender = Arc::new(ChannelUiSender { tx: ui_tx });
            let (layout_tx, layout_rx) = watch::channel(HashMap::new());

            let ctx = ModuleContext::new(
                self.id,
                hub.clone(),
                sm,
                layout_sender,
                display_sender,
                ui_sender,
                layout_rx,
            );
            let port = Box::new(TestModulePort::with_subs(self.vnode, self.subs));

            let resolver = Arc::new(CompositeStyleResolver::new(vec![]));
            let diff_adapter = Arc::new(DefaultVdomDiffAdapter::new());
            let canvas_factory = MockCanvasFactory;

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
                layout_rx: layout_event_rx,
                display_rx,
                ui_rx,
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
        let layout_sender = Arc::new(MockLayoutSender);
        let display_sender = Arc::new(MockDisplaySender);
        let ui_sender = Arc::new(MockUiSender);
        let (_layout_tx, layout_rx) = watch::channel(HashMap::new());

        let mut ctx = ModuleContext::new(
            id,
            hub.clone(),
            sm,
            layout_sender,
            display_sender,
            ui_sender,
            layout_rx,
        )
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
        let mut layout_engines = HashMap::new();
        event_loop.render_all_monitors(&mut layout_engines);

        let event = fixture
            .layout_rx
            .try_recv()
            .expect("Should send size changed command");
        match event {
            LayoutEvent::ModuleSizeChanged {
                monitor_id,
                module_id,
                size,
            } => {
                assert_eq!(monitor_id.as_str(), "DP-1");
                assert_eq!(module_id, id);
                assert_eq!(size.width(), 10);
                assert_eq!(size.height(), 10);
            }
            _ => panic!("Unexpected event"),
        }

        let mut layouts = HashMap::new();
        layouts.insert(
            MonitorId::new("DP-1"),
            Rect::new(Position::new(0, 0), Size::new(10, 10)),
        );
        fixture.layout_tx.send(layouts).unwrap();

        event_loop.render_all_monitors(&mut layout_engines);
        assert!(fixture.layout_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_module_actor_lifecycle() {
        let id = ModuleId::new(2);
        let click_node = VNode::new_rect(
            None,
            None,
            Some(crate::features::vdom::domain::ClickHandlers::from_single(
                crate::features::vdom::domain::UiAction::Exec("render".to_string()),
            )),
            Some(crate::features::vdom::domain::UiAction::Exec(
                "hover".to_string(),
            )),
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
            crate::shared::events::core::InteractionEvent::Pointer(
                crate::shared::events::core::PointerEvent::Click {
                    surface: crate::shared::events::core::SurfaceKind::Bar,
                    button: crate::shared::events::core::PointerButton::Left,
                    pos: crate::shared::primitives::geometry::Position::new(5, 5),
                },
            ),
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
        let mut layout_engines = HashMap::new();
        event_loop.render_all_monitors(&mut layout_engines);

        let event = fixture
            .layout_rx
            .try_recv()
            .expect("Should send size changed command for DP-2");
        match event {
            LayoutEvent::ModuleSizeChanged {
                monitor_id,
                module_id,
                size,
            } => {
                assert_eq!(monitor_id.as_str(), "DP-2");
                assert_eq!(module_id, id);
                assert_eq!(size.width(), 10);
            }
            _ => panic!("Unexpected event"),
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
        let mut layout_engines = HashMap::new();
        event_loop.render_all_monitors(&mut layout_engines);

        assert!(
            event_loop
                .render_pipeline()
                .vdom_trees()
                .contains_key(&MonitorId::new("DP-1"))
        );
        assert!(
            event_loop
                .render_pipeline()
                .render_trees()
                .contains_key(&MonitorId::new("DP-1"))
        );

        // Second run with identical VDOM triggers early continue
        event_loop.render_all_monitors(&mut layout_engines);

        assert!(
            event_loop
                .render_pipeline()
                .vdom_trees()
                .contains_key(&MonitorId::new("DP-1"))
        );
        assert!(
            event_loop
                .render_pipeline()
                .render_trees()
                .contains_key(&MonitorId::new("DP-1"))
        );
    }

    #[tokio::test]
    async fn test_container_module_emits_container_layouts_calculated() {
        let id = ModuleId::new(0);
        let child_node = VNode::new_module(
            crate::shared::primitives::ModuleName::new("clock"),
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
                crate::shared::primitives::ModuleName::new("clock"),
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
        let mut layout_engines = HashMap::new();
        event_loop.render_all_monitors(&mut layout_engines);

        let mut found_container_layouts = false;
        while let Ok(event) = fixture.layout_rx.try_recv() {
            if let LayoutEvent::ContainerLayoutsCalculated {
                parent_id,
                monitor_id,
                layouts,
            } = event
            {
                assert_eq!(parent_id, id);
                assert_eq!(monitor_id.as_str(), "DP-1");
                assert_eq!(layouts.len(), 1);
                assert_eq!(layouts[0].key().name().as_str(), "clock");
                assert_eq!(layouts[0].bounds().width(), 80);
                assert_eq!(layouts[0].bounds().height(), 24);
                found_container_layouts = true;
            }
        }
        assert!(
            found_container_layouts,
            "Should emit ContainerLayoutsCalculated"
        );
    }
}
