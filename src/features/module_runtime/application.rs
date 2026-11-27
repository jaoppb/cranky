//! The module actor and the context it owns: its identity, the signal hub, its surface manager
//! and the channels it sends layout, display and UI commands through.
use crate::features::layout_engine::domain::DisplayCommandSender;
use crate::features::module_runtime::adapters::EventLoop;
use crate::features::module_runtime::domain::{ModuleIdentity, RenderServices};
use crate::features::module_runtime::ports::{AnyModulePort, LayoutEventSender, ModuleHandles};
use crate::features::vdom::domain::UiCommandSender;
use crate::shared::events::signals::SignalHub;
#[cfg(test)]
use crate::shared::primitives::ModuleInstanceId;
use crate::shared::primitives::{ChildBounds, LayoutSurface, ModuleId, MonitorId};
use crate::shared::rendering::ports::canvas::CanvasFactory;
use crate::shared::wayland::ports::DynSurfaceManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::watch;

pub(crate) struct ModuleContext<
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
> {
    identity: ModuleIdentity,
    handles: ModuleHandles<LS, DS, US>,
    layout_rx: watch::Receiver<HashMap<MonitorId, ChildBounds>>,
    pointer_rx: crate::shared::events::core::PointerReceiver,
}

impl<
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
> ModuleContext<LS, DS, US>
{
    pub(crate) fn new(
        id: ModuleId,
        handles: ModuleHandles<LS, DS, US>,
        layout_rx: watch::Receiver<HashMap<MonitorId, ChildBounds>>,
    ) -> Self {
        let pointer_rx = handles.hub().pointer_rx();
        Self {
            identity: ModuleIdentity::new(id),
            handles,
            layout_rx,
            pointer_rx,
        }
    }

    #[must_use]
    pub(crate) fn with_parent(mut self, parent_id: Option<ModuleId>) -> Self {
        self.identity = self.identity.with_parent(parent_id);
        self
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn with_instance_id(mut self, instance_id: Option<ModuleInstanceId>) -> Self {
        self.identity = self.identity.with_instance_id(instance_id);
        self
    }

    #[must_use]
    pub(crate) fn with_surface(mut self, surface: LayoutSurface) -> Self {
        self.identity = self.identity.with_surface(surface);
        self
    }

    #[must_use]
    pub(crate) const fn id(&self) -> ModuleId {
        self.identity.id()
    }

    #[must_use]
    pub(crate) const fn parent_id(&self) -> Option<ModuleId> {
        self.identity.parent_id()
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn instance_id(&self) -> Option<&ModuleInstanceId> {
        self.identity.instance_id()
    }

    #[must_use]
    pub(crate) const fn surface(&self) -> LayoutSurface {
        self.identity.surface()
    }

    #[must_use]
    pub(crate) const fn hub(&self) -> &Arc<SignalHub> {
        self.handles.hub()
    }

    #[must_use]
    pub(crate) const fn surface_manager(&self) -> &DynSurfaceManager {
        self.handles.surface_manager()
    }

    #[must_use]
    pub(crate) fn layout_sender(&self) -> &LS {
        self.handles.senders().layout_sender()
    }

    #[must_use]
    pub(crate) fn display_sender(&self) -> &DS {
        self.handles.senders().display_sender()
    }

    #[must_use]
    pub(crate) fn ui_sender(&self) -> &US {
        self.handles.senders().ui_sender()
    }

    pub(crate) const fn rxs_mut(
        &mut self,
    ) -> (
        &mut watch::Receiver<HashMap<MonitorId, ChildBounds>>,
        &mut crate::shared::events::core::PointerReceiver,
    ) {
        (&mut self.layout_rx, &mut self.pointer_rx)
    }
}

pub(crate) struct ModuleActor<
    F: CanvasFactory + 'static,
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
> {
    port: Box<dyn AnyModulePort>,
    ctx: ModuleContext<LS, DS, US>,
    services: RenderServices<F>,
}

impl<
    F: CanvasFactory + 'static,
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
> ModuleActor<F, LS, DS, US>
{
    #[must_use]
    pub(crate) const fn new(
        port: Box<dyn AnyModulePort>,
        ctx: ModuleContext<LS, DS, US>,
        services: RenderServices<F>,
    ) -> Self {
        Self {
            port,
            ctx,
            services,
        }
    }

    /// Spawns this actor's event loop onto its own task and hands back the
    /// `JoinHandle` instead of discarding it, so a caller that mints this
    /// actor outside the normal config-driven lifecycle (a lazy spawn, see
    /// `ModuleRegistry::spawn_module_lazy`) can track and later abort it.
    pub(crate) fn spawn(self) -> tokio::task::JoinHandle<()> {
        let event_loop = EventLoop::new(self.port, self.ctx, self.services);

        tokio::spawn(async move {
            event_loop.run().await;
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
    use crate::features::layout_engine::domain::DisplayCommand;
    use crate::features::module_runtime::ports::LayoutEvent;
    use crate::features::module_runtime::ports::ModuleSenders;
    use crate::features::module_runtime::test_support::{
        ChannelDisplaySender, ChannelLayoutSender, ChannelUiSender, MockCanvasFactory,
        MockDisplaySender, MockLayoutSender, MockSurfaceManager, MockUiSender, TestModulePort,
    };
    use crate::features::styling::adapters::fs_loader::CompositeStyleResolver;
    use crate::features::vdom::adapters::DefaultVdomDiffAdapter;
    use crate::features::vdom::domain::{ElementSelectors, Interactions, UiCommand, VNode};
    use crate::shared::config::domain::Config;
    use crate::shared::events::signals::{HyprlandState, SignalKind};
    use crate::shared::primitives::SizeConstraint;
    use crate::shared::primitives::geometry::{Position, Rect, Size};

    /// What a test fixture's event loop sends, kept so the channels stay open.
    struct Receivers {
        layout: std::sync::mpsc::Receiver<LayoutEvent>,
        _display: std::sync::mpsc::Receiver<DisplayCommand>,
        _ui: std::sync::mpsc::Receiver<UiCommand>,
    }

    struct TestFixture {
        pub(crate) hub: Arc<SignalHub>,
        pub(crate) sent: Receivers,
        pub(crate) layout_tx: watch::Sender<HashMap<MonitorId, ChildBounds>>,
        pub(crate) event_loop: EventLoop<
            MockCanvasFactory,
            ChannelLayoutSender,
            ChannelDisplaySender,
            ChannelUiSender,
        >,
    }

    struct TestFixtureBuilder {
        id: ModuleId,
        parent_id: Option<ModuleId>,
        monitors: Vec<&'static str>,
        subs: Vec<SignalKind>,
        vnode: VNode,
    }

    /// Hyprland's view of `monitors`, all on workspace 1, the first one focused.
    fn hyprland_with(monitors: &[&'static str]) -> HyprlandState {
        let mut monitors_map = std::collections::BTreeMap::new();
        for monitor in monitors {
            let name = crate::features::workspaces::domain::MonitorName::new(*monitor);
            monitors_map.insert(
                name.clone(),
                crate::features::workspaces::domain::Monitor::new(
                    name,
                    crate::features::workspaces::domain::WorkspaceId::new(1),
                    None,
                ),
            );
        }
        let focused = monitors
            .first()
            .map(|monitor| crate::features::workspaces::domain::MonitorName::new(*monitor));
        HyprlandState::new(std::collections::BTreeMap::new(), monitors_map, focused)
    }

    /// A hub on which Hyprland and Wayland both report `monitors`, if there are any.
    fn hub_with(monitors: &[&'static str]) -> Arc<SignalHub> {
        let hub = Arc::new(SignalHub::new(Config::default()));
        if !monitors.is_empty() {
            hub.hyprland_tx().send(hyprland_with(monitors)).unwrap();

            // Wayland is the actual discovery source - mirror it here so
            // a monitor set up via `with_monitors` is one `EventLoop`
            // will actually render for.
            let mut scales = hub.monitor_scales_rx().borrow().clone();
            for monitor in monitors {
                scales.insert(
                    MonitorId::new(*monitor),
                    crate::shared::primitives::geometry::Scale::new(1.0),
                );
            }
            hub.monitor_scales_tx().send(scales).unwrap();
        }
        hub
    }

    /// Senders writing to channels, with the receiving ends.
    fn channel_senders() -> (
        ModuleSenders<ChannelLayoutSender, ChannelDisplaySender, ChannelUiSender>,
        Receivers,
    ) {
        let (layout_event_tx, layout) = std::sync::mpsc::channel();
        let (display_tx, display) = std::sync::mpsc::channel();
        let (ui_tx, ui_rx) = std::sync::mpsc::channel();
        let senders = ModuleSenders::new(
            Arc::new(ChannelLayoutSender {
                tx: layout_event_tx,
            }),
            Arc::new(ChannelDisplaySender { tx: display_tx }),
            Arc::new(ChannelUiSender { tx: ui_tx }),
        );
        let receivers = Receivers {
            layout,
            _display: display,
            _ui: ui_rx,
        };
        (senders, receivers)
    }

    impl TestFixtureBuilder {
        fn new(id: ModuleId) -> Self {
            Self {
                id,
                parent_id: None,
                monitors: Vec::new(),
                subs: Vec::new(),
                vnode: VNode::new_rect(ElementSelectors::default(), Interactions::default()),
            }
        }

        fn with_parent(mut self, parent_id: ModuleId) -> Self {
            self.parent_id = Some(parent_id);
            self
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
            let hub = hub_with(&self.monitors);
            let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManager);
            let (senders, sent) = channel_senders();
            let (layout_tx, layout_rx) = watch::channel(HashMap::new());
            let handles = ModuleHandles::new(hub.clone(), surface_manager, senders);
            let ctx = ModuleContext::new(self.id, handles, layout_rx).with_parent(self.parent_id);
            let port = Box::new(TestModulePort::with_subs(self.vnode, self.subs));
            let resolver = Arc::new(CompositeStyleResolver::new(vec![]));
            let diff_adapter = Arc::new(DefaultVdomDiffAdapter::new());
            let services = RenderServices::new(MockCanvasFactory, resolver, diff_adapter);
            TestFixture {
                hub,
                sent,
                layout_tx,
                event_loop: EventLoop::new(port, ctx, services),
            }
        }
    }

    #[test]
    fn test_module_context_accessors() {
        let id = ModuleId::new(1);
        let hub = Arc::new(SignalHub::new(Config::default()));
        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManager);
        let layout_sender = Arc::new(MockLayoutSender);
        let display_sender = Arc::new(MockDisplaySender);
        let ui_sender = Arc::new(MockUiSender);
        let (_layout_tx, layout_rx) = watch::channel(HashMap::new());

        let mut ctx = ModuleContext::new(
            id,
            ModuleHandles::new(
                hub.clone(),
                surface_manager,
                ModuleSenders::new(layout_sender, display_sender, ui_sender),
            ),
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

    const ALL_SUBS: [SignalKind; 4] = [
        SignalKind::Time,
        SignalKind::Hyprland,
        SignalKind::Systray,
        SignalKind::Metrics,
    ];

    /// Bounds of `width` x `height` at the origin of `monitor`, with no constraint.
    fn bounds_on(monitor: &str, width: u32, height: u32) -> HashMap<MonitorId, ChildBounds> {
        let rect = Rect::new(Position::new(0, 0), Size::new(width, height));
        HashMap::from([(
            MonitorId::new(monitor),
            ChildBounds::new(rect, SizeConstraint::none()),
        )])
    }

    /// The monitor, module and size of `event`, which must be a size change.
    fn size_change(event: LayoutEvent) -> (MonitorId, ModuleId, Size) {
        let LayoutEvent::ModuleSizeChanged {
            monitor_id,
            module_id,
            size,
            ..
        } = event
        else {
            panic!("Unexpected event");
        };
        (monitor_id, module_id, size)
    }

    #[test]
    fn test_module_actor_measure_and_render_all() {
        crate::test_utils::block_on(async {
            let id = ModuleId::new(1);
            let fixture = TestFixtureBuilder::new(id)
                .with_monitors(&["DP-1"])
                .with_subs(ALL_SUBS.to_vec())
                .build();

            let mut event_loop = fixture.event_loop;
            let mut layout_engines: HashMap<MonitorId, TaffyLayoutAdapter> = HashMap::new();
            event_loop.render_all_monitors(&mut layout_engines);

            let event = fixture.sent.layout.try_recv();
            let (monitor_id, module_id, size) =
                size_change(event.expect("Should send size changed command"));
            assert_eq!(monitor_id.as_str(), "DP-1");
            assert_eq!(module_id, id);
            assert_eq!(size.width(), 10);
            assert_eq!(size.height(), 10);

            fixture.layout_tx.send(bounds_on("DP-1", 10, 10)).unwrap();
            event_loop.render_all_monitors(&mut layout_engines);
            assert!(fixture.sent.layout.try_recv().is_err());
        });
    }

    #[test]
    fn test_suspended_monitor_is_excluded_after_its_rect_is_withdrawn() {
        crate::test_utils::block_on(async {
            // Decision 14: a monitor this *embedded* actor has already rendered
            // on (it's in `render_trees`) must not be rediscovered via the
            // Wayland bootstrap path once its rect disappears — that's what
            // actually stops a child from resurrecting its own surface after its
            // embedding site (a popup, say) closes. A top-level actor (no
            // parent) has no site to lose and is exempt from this — see
            // `test_module_actor_skips_pipeline_when_vdom_unchanged`.
            let fixture = TestFixtureBuilder::new(ModuleId::new(1))
                .with_parent(ModuleId::new(99))
                .with_monitors(&["DP-1"])
                .build();
            let dp1 = MonitorId::new("DP-1");
            let mut event_loop = fixture.event_loop;
            let mut layout_engines: HashMap<MonitorId, TaffyLayoutAdapter> = HashMap::new();

            // First render: DP-1 has never been rendered before, so the
            // Hyprland-sourced bootstrap path discovers and renders it.
            event_loop.render_all_monitors(&mut layout_engines);
            let render_trees = event_loop.render_pipeline().render_trees();
            assert!(render_trees.contains_key(&dp1));
            let first = fixture.sent.layout.try_recv();
            assert!(
                first.is_ok(),
                "first render should report an intrinsic size"
            );

            // No live rect for DP-1 (nothing was ever sent on layout_tx), but
            // DP-1 is now `seen` — discover_monitors must exclude it even
            // though Hyprland still reports it.
            let discovered =
                event_loop.discover_monitors(&HashMap::<MonitorId, ChildBounds>::new());
            assert!(
                !discovered.contains(&dp1),
                "a previously-rendered monitor with no current rect must stay suspended"
            );

            // A second render_all_monitors call (e.g. from some unrelated
            // signal tick) must therefore do nothing for DP-1 — no refresh, no
            // diff, no submit — so `try_recv` sees nothing new.
            event_loop.render_all_monitors(&mut layout_engines);
            assert!(fixture.sent.layout.try_recv().is_err());
        });
    }

    /// A rect clicking which runs `render` and hovering which runs `hover`.
    fn click_node() -> VNode {
        let exec =
            |command: &str| crate::features::vdom::domain::UiAction::Exec(command.to_string());
        let click = crate::features::vdom::domain::ClickHandlers::from_single(exec("render"));
        VNode::new_rect(
            ElementSelectors::default(),
            Interactions::new(Some(click), Some(exec("hover")), None),
        )
    }

    /// A left click at (5, 5) on the bar of DP-1.
    fn bar_click() -> crate::shared::events::core::InteractionEvent {
        crate::shared::events::core::InteractionEvent::Pointer(
            crate::shared::events::core::PointerEvent::Click {
                surface: crate::shared::events::core::SurfaceKind::Bar,
                button: crate::shared::events::core::PointerButton::Left,
                pos: crate::shared::primitives::geometry::Position::new(5, 5),
            },
        )
    }

    #[test]
    fn test_module_actor_lifecycle() {
        crate::test_utils::block_on(async {
            let id = ModuleId::new(2);
            let fixture = TestFixtureBuilder::new(id)
                .with_monitors(&["DP-1"])
                .with_subs(ALL_SUBS.to_vec())
                .with_vnode(click_node())
                .build();

            let actor = fixture.event_loop.into_actor();
            actor.spawn();

            let hub = &fixture.hub;
            hub.time_tx().send(cranky_time::LocalTime::utc(0)).unwrap();
            let systray = crate::features::systray::domain::SystrayState::default();
            hub.systray_tx().send(systray).unwrap();
            let _ = hub
                .pointer_tx()
                .send((id, MonitorId::new("DP-1"), bar_click()));

            tokio::time::sleep(std::time::Duration::from_millis(50)).await;

            drop(fixture.layout_tx);
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        });
    }

    #[test]
    fn test_module_actor_renders_monitor_unknown_to_hyprland() {
        crate::test_utils::block_on(async {
            // Wayland is the sole discovery source: a monitor `monitor_scales`
            // knows about renders even before Hyprland has announced it (the
            // capture that motivated this showed monitoraddedv2 arriving after
            // Hyprland had already moved workspaces onto the new monitor).
            let id = ModuleId::new(1);
            let fixture = TestFixtureBuilder::new(id).build();

            let mut scales = fixture.hub.monitor_scales_rx().borrow().clone();
            scales.insert(
                MonitorId::new("DP-2"),
                crate::shared::primitives::geometry::Scale::new(1.0),
            );
            fixture.hub.monitor_scales_tx().send(scales).unwrap();
            fixture.layout_tx.send(bounds_on("DP-2", 10, 10)).unwrap();

            let mut event_loop = fixture.event_loop;
            let mut layout_engines: HashMap<MonitorId, TaffyLayoutAdapter> = HashMap::new();
            event_loop.render_all_monitors(&mut layout_engines);

            let event = fixture.sent.layout.try_recv();
            let (monitor_id, module_id, size) =
                size_change(event.expect("Should send size changed command for DP-2"));
            assert_eq!(monitor_id.as_str(), "DP-2");
            assert_eq!(module_id, id);
            assert_eq!(size.width(), 10);
        });
    }

    #[test]
    fn test_module_actor_does_not_render_monitor_known_only_via_layout() {
        crate::test_utils::block_on(async {
            // The mirror image of the above: a monitor appearing only in the
            // layout watch (no monitor_scales entry) is not rendered for.
            // Layout is a consequence of Wayland surfaces existing, not a
            // discovery source in its own right.
            let fixture = TestFixtureBuilder::new(ModuleId::new(1)).build();
            fixture.layout_tx.send(bounds_on("DP-2", 10, 10)).unwrap();

            let mut event_loop = fixture.event_loop;
            let mut layout_engines: HashMap<MonitorId, TaffyLayoutAdapter> = HashMap::new();
            event_loop.render_all_monitors(&mut layout_engines);

            assert!(fixture.sent.layout.try_recv().is_err());
        });
    }

    #[test]
    fn test_module_actor_skips_pipeline_when_vdom_unchanged() {
        crate::test_utils::block_on(async {
            let node = VNode::new_text(
                crate::features::vdom::domain::TextContent::new("unchanged".to_string()),
                ElementSelectors::default(),
                Interactions::default(),
            );
            let fixture = TestFixtureBuilder::new(ModuleId::new(1))
                .with_monitors(&["DP-1"])
                .with_vnode(node)
                .build();
            let dp1 = MonitorId::new("DP-1");
            let mut event_loop = fixture.event_loop;
            let mut layout_engines: HashMap<MonitorId, TaffyLayoutAdapter> = HashMap::new();
            event_loop.render_all_monitors(&mut layout_engines);

            let pipeline = event_loop.render_pipeline();
            assert!(pipeline.vdom_trees().contains_key(&dp1));
            assert!(pipeline.render_trees().contains_key(&dp1));

            // Second run with identical VDOM triggers early continue
            event_loop.render_all_monitors(&mut layout_engines);

            let pipeline = event_loop.render_pipeline();
            assert!(pipeline.vdom_trees().contains_key(&dp1));
            assert!(pipeline.render_trees().contains_key(&dp1));
        });
    }

    /// A flex embedding the `clock` module.
    fn clock_container() -> VNode {
        let child_node = VNode::new_module(
            crate::features::vdom::domain::ModuleParams::new(
                crate::shared::primitives::ModuleName::new("clock"),
                None,
                crate::shared::primitives::ModuleOptions::default(),
            ),
            ElementSelectors::default(),
            Interactions::default(),
        );
        VNode::new_flex(
            vec![child_node],
            ElementSelectors::default(),
            Interactions::default(),
        )
    }

    /// Child sizes in which `clock` reported 80x24 on DP-1.
    fn clock_sizes() -> HashMap<MonitorId, crate::shared::primitives::ChildSizesMap> {
        let mut mon_map = crate::shared::primitives::ChildSizesMap::new();
        mon_map.insert(
            crate::shared::primitives::ModuleKey::from_name(
                crate::shared::primitives::ModuleName::new("clock"),
            ),
            Size::new(80, 24),
        );
        HashMap::from([(MonitorId::new("DP-1"), mon_map)])
    }

    /// The parent, monitor and child layouts of the first `ContainerLayoutsCalculated` sent.
    fn container_layouts(
        sent: &std::sync::mpsc::Receiver<LayoutEvent>,
    ) -> Option<(
        ModuleId,
        MonitorId,
        Vec<crate::shared::primitives::ChildModuleLayout>,
    )> {
        sent.try_iter().find_map(|event| {
            if let LayoutEvent::ContainerLayoutsCalculated {
                parent_id,
                monitor_id,
                layouts,
            } = event
            {
                Some((parent_id, monitor_id, layouts))
            } else {
                None
            }
        })
    }

    #[test]
    fn test_container_module_emits_container_layouts_calculated() {
        crate::test_utils::block_on(async {
            let id = ModuleId::new(0);
            let fixture = TestFixtureBuilder::new(id)
                .with_monitors(&["DP-1"])
                .with_vnode(clock_container())
                .build();
            // Report child module size in hub
            fixture.hub.module_sizes_tx().send(clock_sizes()).unwrap();
            fixture.layout_tx.send(bounds_on("DP-1", 1920, 30)).unwrap();

            let mut event_loop = fixture.event_loop;
            let mut layout_engines: HashMap<MonitorId, TaffyLayoutAdapter> = HashMap::new();
            event_loop.render_all_monitors(&mut layout_engines);

            let (parent_id, monitor_id, layouts) = container_layouts(&fixture.sent.layout)
                .expect("Should emit ContainerLayoutsCalculated");
            assert_eq!(parent_id, id);
            assert_eq!(monitor_id.as_str(), "DP-1");
            assert_eq!(layouts.len(), 1);
            assert_eq!(layouts[0].key().name().as_str(), "clock");
            assert_eq!(layouts[0].bounds().rect().width(), 80);
            assert_eq!(layouts[0].bounds().rect().height(), 24);
        });
    }
}
