//! Tests of the application actor.

mod display;
mod fakes;
mod fixtures;
mod interface;
mod layout;
mod lazy_spawn;
mod reload;
mod run_loop;

use super::*;
use crate::app::test_fakes::{FakeDisplayServer, FakeRegistry, FakeSni};
use crate::features::module_runtime::ports::{
    ModuleHandles, ModuleRuntimeDependencies, ModuleSenders,
};
use crate::features::module_runtime::test_support::MockSurfaceManager;
use crate::shared::primitives::geometry::{BarWidth, Position, Rect, Size};
use crate::shared::wayland::ports::DynSurfaceManager;
use crate::shared::wayland::ports::MonitorBar;
use fixtures::{app_over, config_sampling_every_2500_ms};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::sync::mpsc;

/// A registry holding a root `bar` (id 0) that embeds one `clock` (id 1).
fn bar_embedding_clock() -> FakeRegistry {
    let mut registry = FakeRegistry::default();
    registry.root_module = Some(ModuleId::new(0));
    registry.module_ids = vec![ModuleId::new(0), ModuleId::new(1)];
    let mut names = HashMap::new();
    names.insert(
        ModuleId::new(0),
        crate::shared::primitives::ModuleName::new("bar"),
    );
    names.insert(
        ModuleId::new(1),
        crate::shared::primitives::ModuleName::new("clock"),
    );
    registry.module_names = names;
    let mut name_to_ids = HashMap::new();
    name_to_ids.insert(
        crate::shared::primitives::ModuleName::new("bar"),
        vec![ModuleId::new(0)],
    );
    name_to_ids.insert(
        crate::shared::primitives::ModuleName::new("clock"),
        vec![ModuleId::new(1)],
    );
    registry.name_to_ids = name_to_ids;
    registry.resolve_site = Box::new(|parent, key| {
        if parent == Some(ModuleId::new(0)) && key.name().as_str() == "clock" {
            Some(ModuleId::new(1))
        } else {
            None
        }
    });
    registry
}

/// The bar reporting where it laid its clock out on `monitor`.
fn clock_laid_out_on(monitor: &str, x: i32) -> LayoutEvent {
    LayoutEvent::ContainerLayoutsCalculated {
        parent_id: ModuleId::new(0),
        monitor_id: MonitorId::new(monitor),
        layouts: vec![crate::shared::primitives::ChildModuleLayout::new(
            crate::shared::primitives::ModuleKey::from_name(
                crate::shared::primitives::ModuleName::new("clock"),
            ),
            crate::shared::primitives::ChildBounds::new(
                Rect::new(Position::new(x, 0), Size::new(80, 24)),
                crate::shared::primitives::SizeConstraint::none(),
            ),
            crate::shared::primitives::LayoutSurface::Bar,
        )],
    }
}

/// Queues one command of every kind `CrankyApp::run` dispatches.
async fn queue_one_of_each_command(
    display_tx: &mpsc::Sender<DisplayCommand>,
    layout_tx: &mpsc::Sender<LayoutEvent>,
    ui_tx: &mpsc::Sender<UiCommand>,
) {
    display_tx
        .send(DisplayCommand::RequestRender)
        .await
        .unwrap();
    layout_tx
        .send(LayoutEvent::ModuleSizeChanged {
            monitor_id: MonitorId::new("1"),
            module_id: ModuleId::new(1),
            size: Size::new(10, 10),
            settled_width: None,
        })
        .await
        .unwrap();
    display_tx
        .send(DisplayCommand::ShowFloatingSurface(
            crate::features::layout_engine::domain::ShowFloating {
                kind: crate::features::layout_engine::domain::FloatingKind::Tooltip,
                monitor_id: None,
                anchor_rect: None,
                buffer: crate::shared::primitives::render::RenderBuffer::new(
                    vec![0u8; 4],
                    Size::new(1, 1),
                ),
                logical_size: Size::new(1, 1),
                offset: None,
                parent: None,
            },
        ))
        .await
        .unwrap();
    display_tx
        .send(DisplayCommand::HideFloatingSurface {
            kind: crate::features::layout_engine::domain::FloatingKind::Tooltip,
        })
        .await
        .unwrap();
    ui_tx
        .send(UiCommand::SystrayAction {
            id: "a".into(),
            action: "b".into(),
            pos: None,
        })
        .await
        .unwrap();
}

#[test]
fn test_app_initialization() {
    crate::test_utils::block_on(async {
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config));
        let (display_tx, display_rx) = mpsc::channel(32);
        let (layout_tx, layout_rx) = mpsc::channel(32);
        let (ui_tx, ui_rx) = mpsc::channel(32);
        let (_system_tx, system_rx) = mpsc::channel(32);

        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManager);

        let fake_registry = FakeRegistry::default();

        let canvas_factory =
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new();

        let display_sender = Arc::new(display_tx);
        let layout_sender = Arc::new(layout_tx);
        let ui_sender = Arc::new(ui_tx);

        let deps = ModuleRuntimeDependencies::new(
            ModuleHandles::new(
                hub,
                surface_manager,
                ModuleSenders::new(layout_sender, display_sender, ui_sender),
            ),
            canvas_factory,
        );
        let channels = StateChannels::new(
            ModuleReceivers::new(layout_rx, display_rx, ui_rx),
            system_rx,
        );
        let app_result = CrankyApp::new(deps, channels, Box::new(fake_registry));

        assert!(app_result.is_ok());
    });
}

#[test]
fn test_app_run_exit_on_display_error() {
    crate::test_utils::block_on(async {
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config));
        let (display_tx, display_rx) = mpsc::channel(32);
        let (layout_tx, layout_rx) = mpsc::channel(32);
        let (ui_tx, ui_rx) = mpsc::channel(32);
        let (_system_tx, system_rx) = mpsc::channel(32);

        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManager);

        let fake_registry = FakeRegistry::default();

        let canvas_factory =
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new();

        let display_sender = Arc::new(display_tx);
        let layout_sender = Arc::new(layout_tx);
        let ui_sender = Arc::new(ui_tx);

        let deps = ModuleRuntimeDependencies::new(
            ModuleHandles::new(
                hub.clone(),
                surface_manager,
                ModuleSenders::new(layout_sender, display_sender, ui_sender),
            ),
            canvas_factory,
        );
        let channels = StateChannels::new(
            ModuleReceivers::new(layout_rx, display_rx, ui_rx),
            system_rx,
        );
        let mut app = CrankyApp::new(deps, channels, Box::new(fake_registry)).unwrap();

        let fake_display = FakeDisplayServer::failing("Test error");

        let disconnected_bus = crate::shared::dbus::ports::DisconnectedBus;
        let dbus_manager = crate::shared::dbus::subscription_manager::DbusSubscriptionManager::new(
            std::sync::Arc::new(disconnected_bus),
            &hub,
        );
        let fake_sni = FakeSni;

        let result = app
            .run(fake_display, BusAdapters::new(dbus_manager, fake_sni))
            .await;
        assert!(result.is_err());
    });
}

#[test]
fn test_calculate_layout_unfocused() {
    let unfocused = crate::shared::config::domain::PartialRootConfig::new(
        crate::shared::config::domain::CreatePartialRootConfigCommand::new(
            Some(crate::shared::primitives::geometry::BarHeight::new(20)),
            None,
            None,
        ),
    );
    let mut opts_map = HashMap::new();
    opts_map.insert(
        "center".to_string(),
        crate::shared::primitives::DynamicValue::Array(vec![
            crate::shared::primitives::DynamicValue::String("clock".to_string()),
        ]),
    );
    let opts = crate::shared::primitives::ModuleOptions::new(opts_map);

    let root_config = crate::shared::config::domain::RootConfig::default()
        .with_unfocused(Some(unfocused))
        .with_options(opts);

    let config = Config::default().with_root(root_config);

    let mut name_to_ids = HashMap::new();
    name_to_ids.insert(
        crate::shared::primitives::ModuleName::new("clock"),
        vec![crate::shared::primitives::ModuleId::new(1)],
    );

    let read_model = AppReadModel {
        config: config.clone(),
        root_module: None,
        module_ids: vec![crate::shared::primitives::ModuleId::new(1)],
        module_names: {
            let mut names = HashMap::new();
            names.insert(
                crate::shared::primitives::ModuleId::new(1),
                crate::shared::primitives::ModuleName::new("clock"),
            );
            names
        },
        name_to_ids,
        module_keys: HashMap::new(),
        module_sizes: {
            let mut sizes_by_monitor = HashMap::new();
            let mut monitor_sizes = HashMap::new();
            monitor_sizes.insert(
                crate::shared::primitives::ModuleId::new(1),
                crate::shared::primitives::geometry::Size::new(50, 10),
            );
            sizes_by_monitor.insert(MonitorId::new("DP-1"), monitor_sizes);
            sizes_by_monitor
        },
        computed_layouts: HashMap::new(),
    };

    let monitor_1 = MonitorId::new("DP-1");

    // 1. Calculate with focused config
    let layouts_focused = read_model.calculate_layout(
        &MonitorBar::new(monitor_1.clone(), BarWidth::new(1920)),
        config.root(),
    );
    assert_eq!(layouts_focused.len(), 1);
    let focused_layout = &layouts_focused[0];

    // height 30, available height = 30, module height = 10, y should be (30 - 10) / 2 = 10
    assert_eq!(focused_layout.bounds().position().y(), 10);

    // 2. Calculate with unfocused config
    let unfocused_root = config.root().as_unfocused();
    let layouts_unfocused = read_model.calculate_layout(
        &MonitorBar::new(monitor_1, BarWidth::new(1920)),
        &unfocused_root,
    );
    assert_eq!(layouts_unfocused.len(), 1);
    let unfocused_layout = &layouts_unfocused[0];

    // height 20, available height = 20, module height = 10, y should be (20 - 10) / 2 = 5
    assert_eq!(unfocused_layout.bounds().position().y(), 5);
}

#[test]
fn test_app_error_fmt() {
    let err1 = AppError::Module(
        crate::features::module_runtime::ports::RegistryLoadError::ModuleNotFound("test".into()),
    );
    assert_eq!(err1.to_string(), "Module error: Module not found: test");
    let err2 = AppError::Internal {
        message: "test".into(),
    };
    assert_eq!(err2.to_string(), "Internal error: test");
}

#[test]
fn test_calculate_layout_left_right() {
    let mut opts_map = HashMap::new();
    opts_map.insert(
        "left".to_string(),
        crate::shared::primitives::DynamicValue::Array(vec![
            crate::shared::primitives::DynamicValue::String("m1".to_string()),
            crate::shared::primitives::DynamicValue::String("m2".to_string()),
        ]),
    );
    opts_map.insert(
        "right".to_string(),
        crate::shared::primitives::DynamicValue::Array(vec![
            crate::shared::primitives::DynamicValue::String("m3".to_string()),
        ]),
    );
    let opts = crate::shared::primitives::ModuleOptions::new(opts_map);

    let root_config = crate::shared::config::domain::RootConfig::default().with_options(opts);

    let config = Config::default().with_root(root_config);

    let mut name_to_ids = HashMap::new();
    name_to_ids.insert(
        crate::shared::primitives::ModuleName::new("m1"),
        vec![ModuleId::new(1)],
    );
    name_to_ids.insert(
        crate::shared::primitives::ModuleName::new("m2"),
        vec![ModuleId::new(2)],
    );
    name_to_ids.insert(
        crate::shared::primitives::ModuleName::new("m3"),
        vec![ModuleId::new(3)],
    );

    let mut read_model = AppReadModel {
        config: config.clone(),
        root_module: None,
        module_ids: vec![ModuleId::new(1), ModuleId::new(2), ModuleId::new(3)],
        module_names: HashMap::new(),
        name_to_ids,
        module_keys: HashMap::new(),
        module_sizes: HashMap::new(),
        computed_layouts: HashMap::new(),
    };

    let mut sizes = HashMap::new();
    sizes.insert(ModuleId::new(1), Size::new(100, 20));
    sizes.insert(ModuleId::new(2), Size::new(50, 20));
    sizes.insert(ModuleId::new(3), Size::new(80, 20));
    read_model
        .module_sizes
        .insert(MonitorId::new("DP-1"), sizes);

    let layouts = read_model.calculate_layout(
        &MonitorBar::new(MonitorId::new("DP-1"), BarWidth::new(1920)),
        config.root(),
    );
    assert_eq!(layouts.len(), 3);

    let gap = 8;
    let padding_h = 8;

    let first_layout = layouts
        .iter()
        .find(|layout| layout.id() == ModuleId::new(1))
        .unwrap();
    assert_eq!(first_layout.bounds().x(), padding_h);

    let second_layout = layouts
        .iter()
        .find(|layout| layout.id() == ModuleId::new(2))
        .unwrap();
    assert_eq!(second_layout.bounds().x(), padding_h + 100 + gap);

    let third_layout = layouts
        .iter()
        .find(|layout| layout.id() == ModuleId::new(3))
        .unwrap();
    assert_eq!(third_layout.bounds().x(), 1920 - padding_h - 80);
}

#[test]
fn test_app_run_commands_and_signals() {
    crate::test_utils::block_on(async {
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config));
        let (display_tx, display_rx) = mpsc::channel(32);
        let (layout_tx, layout_rx) = mpsc::channel(32);
        let (ui_tx, ui_rx) = mpsc::channel(32);
        let (_system_tx, system_rx) = mpsc::channel(32);

        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManager);

        let fake_registry = FakeRegistry::default();

        let canvas_factory =
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new();

        let display_sender = Arc::new(display_tx.clone());
        let layout_sender = Arc::new(layout_tx.clone());
        let ui_sender = Arc::new(ui_tx.clone());

        let deps = ModuleRuntimeDependencies::new(
            ModuleHandles::new(
                hub.clone(),
                surface_manager,
                ModuleSenders::new(layout_sender, display_sender, ui_sender),
            ),
            canvas_factory,
        );
        let channels = StateChannels::new(
            ModuleReceivers::new(layout_rx, display_rx, ui_rx),
            system_rx,
        );
        let mut app = CrankyApp::new(deps, channels, Box::new(fake_registry)).unwrap();

        // Let it succeed twice, then fail to exit the loop
        let mut call_count = 0;
        let fake_display = FakeDisplayServer {
            wait_for_events: Box::new(move || {
                call_count += 1;
                if call_count <= 2 {
                    Box::pin(std::future::ready(Ok(())))
                } else {
                    Box::pin(std::future::ready(Err(
                        crate::shared::wayland::ports::DisplayServerError::Internal("Exit".into()),
                    )))
                }
            }),
            renders: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        };

        let disconnected_bus = crate::shared::dbus::ports::DisconnectedBus;
        let dbus_manager = crate::shared::dbus::subscription_manager::DbusSubscriptionManager::new(
            std::sync::Arc::new(disconnected_bus),
            &hub,
        );
        let fake_sni = FakeSni;

        queue_one_of_each_command(&display_tx, &layout_tx, &ui_tx).await;

        // Trigger config, hyprland, and monitor_scales changes
        hub.config_tx().send(Config::default()).unwrap();
        hub.hyprland_tx()
            .send(crate::shared::events::signals::HyprlandState::new(
                std::collections::BTreeMap::new(),
                std::collections::BTreeMap::new(),
                Some(crate::features::workspaces::domain::MonitorName::new("1")),
            ))
            .unwrap();
        let mut scales = hub.monitor_scales_rx().borrow().clone();
        scales.insert(
            MonitorId::new("1"),
            crate::shared::primitives::geometry::Scale::new(1.0),
        );
        hub.monitor_scales_tx().send(scales).unwrap();

        let result = app
            .run(fake_display, BusAdapters::new(dbus_manager, fake_sni))
            .await;
        assert!(result.is_err());
    });
}

#[test]
fn test_container_layouts_calculated_preserves_multi_monitors() {
    crate::test_utils::block_on(async {
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config));
        let (display_tx, display_rx) = mpsc::channel(32);
        let (layout_tx, layout_rx) = mpsc::channel(32);
        let (ui_tx, ui_rx) = mpsc::channel(32);
        let (_system_tx, system_rx) = mpsc::channel(32);
        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManager);

        let (bar_sender, _bar_layouts) = tokio::sync::watch::channel(HashMap::new());
        let (clock_sender, mut clock_layouts) = tokio::sync::watch::channel(HashMap::new());
        let mut fake_registry = bar_embedding_clock();
        fake_registry.senders = HashMap::from([
            (
                ModuleId::new(0),
                crate::app::registry::WatchLayoutSender::new(bar_sender),
            ),
            (
                ModuleId::new(1),
                crate::app::registry::WatchLayoutSender::new(clock_sender),
            ),
        ]);

        let canvas_factory =
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new();

        let display_sender = Arc::new(display_tx);
        let layout_sender = Arc::new(layout_tx.clone());
        let ui_sender = Arc::new(ui_tx);

        let deps = ModuleRuntimeDependencies::new(
            ModuleHandles::new(
                hub.clone(),
                surface_manager,
                ModuleSenders::new(layout_sender, display_sender, ui_sender),
            ),
            canvas_factory,
        );
        let channels = StateChannels::new(
            ModuleReceivers::new(layout_rx, display_rx, ui_rx),
            system_rx,
        );
        let mut app = CrankyApp::new(deps, channels, Box::new(fake_registry)).unwrap();

        // 1. Send ContainerLayoutsCalculated for DP-1
        layout_tx
            .send(clock_laid_out_on("DP-1", 100))
            .await
            .unwrap();

        // 2. Send ContainerLayoutsCalculated for DP-2
        layout_tx
            .send(clock_laid_out_on("DP-2", 150))
            .await
            .unwrap();

        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
        let fake_display = FakeDisplayServer {
            wait_for_events: Box::new(move || {
                let mut stop_rx = stop_rx.clone();
                Box::pin(async move {
                    let _ = stop_rx.changed().await;
                    Err(crate::shared::wayland::ports::DisplayServerError::Internal(
                        "Done".to_string(),
                    ))
                })
            }),
            renders: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        };

        let disconnected_bus = crate::shared::dbus::ports::DisconnectedBus;
        let dbus_manager = crate::shared::dbus::subscription_manager::DbusSubscriptionManager::new(
            std::sync::Arc::new(disconnected_bus),
            &hub,
        );
        let fake_sni = FakeSni;

        // Run app in background task and verify clock_layouts gets both DP-1 and DP-2
        let app_handle = tokio::spawn(async move {
            let _ = app
                .run(fake_display, BusAdapters::new(dbus_manager, fake_sni))
                .await;
        });

        // Wait for clock_layouts to see both DP-1 and DP-2
        let mut attempts = 0;
        loop {
            clock_layouts.changed().await.unwrap();
            let current = clock_layouts.borrow().clone();
            if current.contains_key(&MonitorId::new("DP-1"))
                && current.contains_key(&MonitorId::new("DP-2"))
            {
                assert_eq!(current[&MonitorId::new("DP-1")].rect().x(), 100);
                assert_eq!(current[&MonitorId::new("DP-2")].rect().x(), 150);
                break;
            }
            attempts += 1;
            assert!(
                attempts <= 10,
                "Did not receive both DP-1 and DP-2 bounds in child layout_rx"
            );
        }

        let _ = stop_tx.send(true);
        let _ = app_handle.await;
    });
}

#[test]
fn test_handle_container_layouts_does_not_cross_talk_between_sites() {
    crate::test_utils::block_on(async {
        // Two different parents (10 and 20) each embed a module named
        // "calendar" — resolve_site must route each parent's layout event to
        // its own site's ModuleId, never the other parent's.
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config));
        let (display_tx, display_rx) = mpsc::channel(32);
        let (layout_tx, layout_rx) = mpsc::channel(32);
        let (ui_tx, ui_rx) = mpsc::channel(32);
        let (_system_tx, system_rx) = mpsc::channel(32);
        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManager);

        let parent_10 = ModuleId::new(10);
        let parent_20 = ModuleId::new(20);
        let site_a = ModuleId::new(11);
        let site_b = ModuleId::new(21);

        let mut fake_registry = FakeRegistry::default();
        fake_registry.resolve_site = Box::new(move |parent, key| {
            if key.name().as_str() != "calendar" {
                return None;
            }
            if parent == Some(parent_10) {
                Some(site_a)
            } else if parent == Some(parent_20) {
                Some(site_b)
            } else {
                None
            }
        });
        // Each site's own parent — consulted when the *other* parent's
        // render touches the same monitor, to confirm `site_a`/`site_b`
        // belong to a different parent and so must not be suspended.
        fake_registry.parent_of = Box::new(move |id| {
            if id == site_a {
                Some(parent_10)
            } else if id == site_b {
                Some(parent_20)
            } else {
                None
            }
        });

        let canvas_factory =
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new();

        let deps = ModuleRuntimeDependencies::new(
            ModuleHandles::new(
                hub,
                surface_manager,
                ModuleSenders::new(Arc::new(layout_tx), Arc::new(display_tx), Arc::new(ui_tx)),
            ),
            canvas_factory,
        );
        let channels = StateChannels::new(
            ModuleReceivers::new(layout_rx, display_rx, ui_rx),
            system_rx,
        );
        let mut app = CrankyApp::new(deps, channels, Box::new(fake_registry)).unwrap();

        let monitor = MonitorId::new("DP-1");
        let key = crate::shared::primitives::ModuleKey::from_name(
            crate::shared::primitives::ModuleName::new("calendar"),
        );

        app.handle_container_layouts(
            ModuleOnMonitor {
                module: parent_10,
                monitor: &monitor,
            },
            &[crate::shared::primitives::ChildModuleLayout::new(
                key.clone(),
                crate::shared::primitives::ChildBounds::new(
                    Rect::new(Position::new(10, 0), Size::new(50, 20)),
                    crate::shared::primitives::SizeConstraint::none(),
                ),
                crate::shared::primitives::LayoutSurface::Bar,
            )],
        )
        .await;
        app.handle_container_layouts(
            ModuleOnMonitor {
                module: parent_20,
                monitor: &monitor,
            },
            &[crate::shared::primitives::ChildModuleLayout::new(
                key,
                crate::shared::primitives::ChildBounds::new(
                    Rect::new(Position::new(90, 0), Size::new(50, 20)),
                    crate::shared::primitives::SizeConstraint::none(),
                ),
                crate::shared::primitives::LayoutSurface::Bar,
            )],
        )
        .await;

        let layouts = &app.read_model.computed_layouts[&monitor];
        assert_eq!(layouts[&site_a].rect().x(), 10);
        assert_eq!(layouts[&site_b].rect().x(), 90);
        assert_eq!(layouts.len(), 2);
    });
}

#[test]
fn test_handle_container_layouts_suspends_child_whose_site_closed() {
    crate::test_utils::block_on(async {
        // Decision 14: once a parent's render no longer embeds a child on a
        // monitor (its popup closed), the stale `computed_layouts` entry
        // must be cleared and the child told directly — this is what stops
        // it from rendering there again (gap 8).
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config));
        let (display_tx, display_rx) = mpsc::channel(32);
        let (layout_tx, layout_rx) = mpsc::channel(32);
        let (ui_tx, ui_rx) = mpsc::channel(32);
        let (_system_tx, system_rx) = mpsc::channel(32);
        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManager);

        let parent = ModuleId::new(1);
        let child = ModuleId::new(2);

        let mut fake_registry = FakeRegistry::default();
        fake_registry.resolve_site = Box::new(move |requested_parent, _| {
            if requested_parent == Some(parent) {
                Some(child)
            } else {
                None
            }
        });
        fake_registry.parent_of = Box::new(move |id| if id == child { Some(parent) } else { None });

        let canvas_factory =
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new();
        let deps = ModuleRuntimeDependencies::new(
            ModuleHandles::new(
                hub,
                surface_manager,
                ModuleSenders::new(Arc::new(layout_tx), Arc::new(display_tx), Arc::new(ui_tx)),
            ),
            canvas_factory,
        );
        let channels = StateChannels::new(
            ModuleReceivers::new(layout_rx, display_rx, ui_rx),
            system_rx,
        );
        let mut app = CrankyApp::new(deps, channels, Box::new(fake_registry)).unwrap();

        let (child_layout_tx, child_layout_rx) = tokio::sync::watch::channel(HashMap::new());
        app.layout_senders.insert(
            child,
            crate::app::registry::WatchLayoutSender::new(child_layout_tx),
        );

        let monitor = MonitorId::new("DP-1");
        let key = crate::shared::primitives::ModuleKey::from_name(
            crate::shared::primitives::ModuleName::new("calendar"),
        );

        // Popup open: the child gets a rect on DP-1.
        app.handle_container_layouts(
            ModuleOnMonitor {
                module: parent,
                monitor: &monitor,
            },
            &[crate::shared::primitives::ChildModuleLayout::new(
                key,
                crate::shared::primitives::ChildBounds::new(
                    Rect::new(Position::new(0, 0), Size::new(50, 20)),
                    crate::shared::primitives::SizeConstraint::none(),
                ),
                crate::shared::primitives::LayoutSurface::Popup,
            )],
        )
        .await;
        assert!(
            app.read_model
                .computed_layouts
                .get(&monitor)
                .is_some_and(|monitor_layouts| monitor_layouts.contains_key(&child))
        );
        assert!(child_layout_rx.borrow().contains_key(&monitor));

        // Popup closed: the parent's next render carries no children at all.
        app.handle_container_layouts(
            ModuleOnMonitor {
                module: parent,
                monitor: &monitor,
            },
            &[],
        )
        .await;

        assert!(
            !app.read_model
                .computed_layouts
                .get(&monitor)
                .is_some_and(|monitor_layouts| monitor_layouts.contains_key(&child)),
            "the stale entry must be cleared once the site closes"
        );
        assert!(
            !child_layout_rx.borrow().contains_key(&monitor),
            "the child must be told its monitor is gone, not left with a stale rect"
        );
    });
}

#[test]
fn test_ensure_lazy_spawn_rejects_self_cycle() {
    crate::test_utils::block_on(async {
        // A parent named "calendar" embedding another "calendar" would
        // create a cycle — must be rejected and recorded without ever
        // calling `spawn_module`.
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config));
        let hub_check = hub.clone();
        let (display_tx, display_rx) = mpsc::channel(32);
        let (layout_tx, layout_rx) = mpsc::channel(32);
        let (ui_tx, ui_rx) = mpsc::channel(32);
        let (_system_tx, system_rx) = mpsc::channel(32);
        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManager);

        let parent = ModuleId::new(1);
        let mut names = HashMap::new();
        names.insert(
            parent,
            crate::shared::primitives::ModuleName::new("calendar"),
        );
        let mut keys = HashMap::new();
        keys.insert(
            parent,
            crate::shared::primitives::ModuleKey::from_name(
                crate::shared::primitives::ModuleName::new("calendar"),
            ),
        );

        let mut fake_registry = FakeRegistry::default();
        fake_registry.module_names = names;
        fake_registry.module_keys = keys;
        fake_registry.resolve_site = Box::new(|_, _| None);

        let canvas_factory =
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new();
        let deps = ModuleRuntimeDependencies::new(
            ModuleHandles::new(
                hub,
                surface_manager,
                ModuleSenders::new(Arc::new(layout_tx), Arc::new(display_tx), Arc::new(ui_tx)),
            ),
            canvas_factory,
        );
        let spawn_attempts = fake_registry.spawn_module_calls.clone();
        let channels = StateChannels::new(
            ModuleReceivers::new(layout_rx, display_rx, ui_rx),
            system_rx,
        );
        let mut app = CrankyApp::new(deps, channels, Box::new(fake_registry)).unwrap();

        let monitor = MonitorId::new("DP-1");
        let key = crate::shared::primitives::ModuleKey::from_name(
            crate::shared::primitives::ModuleName::new("calendar"),
        );
        app.handle_container_layouts(
            ModuleOnMonitor {
                module: parent,
                monitor: &monitor,
            },
            &[crate::shared::primitives::ChildModuleLayout::new(
                key,
                crate::shared::primitives::ChildBounds::new(
                    Rect::new(Position::new(0, 0), Size::new(50, 20)),
                    crate::shared::primitives::SizeConstraint::none(),
                ),
                crate::shared::primitives::LayoutSurface::Bar,
            )],
        )
        .await;

        let site = crate::shared::primitives::ModuleSite::new(
            Some(parent),
            crate::shared::primitives::ModuleKey::from_name(
                crate::shared::primitives::ModuleName::new("calendar"),
            ),
        );
        assert!(hub_check.module_errors_rx().borrow().contains_key(&site));
        assert_eq!(spawn_attempts.load(Ordering::SeqCst), 0);
        assert!(app.read_model.computed_layouts.is_empty());
    });
}

#[test]
fn test_ensure_lazy_spawn_does_not_retry_after_recorded_failure() {
    crate::test_utils::block_on(async {
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config));
        let hub_check = hub.clone();
        let (display_tx, display_rx) = mpsc::channel(32);
        let (layout_tx, layout_rx) = mpsc::channel(32);
        let (ui_tx, ui_rx) = mpsc::channel(32);
        let (_system_tx, system_rx) = mpsc::channel(32);
        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManager);

        let parent = ModuleId::new(1);
        let mut names = HashMap::new();
        names.insert(parent, crate::shared::primitives::ModuleName::new("bar"));

        let mut fake_registry = FakeRegistry::default();
        fake_registry.module_names = names;
        fake_registry.resolve_site = Box::new(|_, _| None);
        fake_registry.parent_of = Box::new(|_| None);
        let spawn_attempts = fake_registry.spawn_module_calls.clone();

        let canvas_factory =
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new();
        let deps = ModuleRuntimeDependencies::new(
            ModuleHandles::new(
                hub,
                surface_manager,
                ModuleSenders::new(Arc::new(layout_tx), Arc::new(display_tx), Arc::new(ui_tx)),
            ),
            canvas_factory,
        );
        let channels = StateChannels::new(
            ModuleReceivers::new(layout_rx, display_rx, ui_rx),
            system_rx,
        );
        let mut app = CrankyApp::new(deps, channels, Box::new(fake_registry)).unwrap();

        let monitor = MonitorId::new("DP-1");
        let key = crate::shared::primitives::ModuleKey::from_name(
            crate::shared::primitives::ModuleName::new("calendarr"),
        );
        let layouts = [crate::shared::primitives::ChildModuleLayout::new(
            key,
            crate::shared::primitives::ChildBounds::new(
                Rect::new(Position::new(0, 0), Size::new(50, 20)),
                crate::shared::primitives::SizeConstraint::none(),
            ),
            crate::shared::primitives::LayoutSurface::Bar,
        )];

        // First attempt: spawn_module is called and fails, and the failure
        // is recorded.
        app.handle_container_layouts(
            ModuleOnMonitor {
                module: parent,
                monitor: &monitor,
            },
            &layouts,
        )
        .await;
        let site = crate::shared::primitives::ModuleSite::new(
            Some(parent),
            crate::shared::primitives::ModuleKey::from_name(
                crate::shared::primitives::ModuleName::new("calendarr"),
            ),
        );
        assert!(hub_check.module_errors_rx().borrow().contains_key(&site));
        assert_eq!(spawn_attempts.load(Ordering::SeqCst), 1);

        // Second identical attempt: spawn_module must not be called again.
        app.handle_container_layouts(
            ModuleOnMonitor {
                module: parent,
                monitor: &monitor,
            },
            &layouts,
        )
        .await;
        assert_eq!(spawn_attempts.load(Ordering::SeqCst), 1);
    });
}

/// A lazy spawn hands the registry the parent, the full key, the options and the floating
/// surface of the site the parent laid its child out at.
#[test]
fn test_ensure_lazy_spawn_passes_the_site_options_and_surface_to_the_registry() {
    crate::test_utils::block_on(async {
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config));
        let (display_tx, display_rx) = mpsc::channel(32);
        let (layout_tx, layout_rx) = mpsc::channel(32);
        let (ui_tx, ui_rx) = mpsc::channel(32);
        let (_system_tx, system_rx) = mpsc::channel(32);
        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManager);

        let parent = ModuleId::new(1);
        let fake_registry = FakeRegistry::default();
        let requests = fake_registry.spawn_requests.clone();

        let canvas_factory =
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new();
        let deps = ModuleRuntimeDependencies::new(
            ModuleHandles::new(
                hub,
                surface_manager,
                ModuleSenders::new(Arc::new(layout_tx), Arc::new(display_tx), Arc::new(ui_tx)),
            ),
            canvas_factory,
        );
        let channels = StateChannels::new(
            ModuleReceivers::new(layout_rx, display_rx, ui_rx),
            system_rx,
        );
        let mut app = CrankyApp::new(deps, channels, Box::new(fake_registry)).unwrap();

        let key = crate::shared::primitives::ModuleKey::new(
            crate::shared::primitives::ModuleName::new("calendar"),
            Some(crate::shared::primitives::ModuleInstanceId::new("cal-2")),
        );
        let options = crate::shared::primitives::ModuleOptions::new(HashMap::from([(
            "format".to_string(),
            crate::shared::primitives::DynamicValue::String("%A %d".to_string()),
        )]));
        let layouts = [crate::shared::primitives::ChildModuleLayout::new(
            key.clone(),
            crate::shared::primitives::ChildBounds::new(
                Rect::new(Position::new(0, 0), Size::new(50, 20)),
                crate::shared::primitives::SizeConstraint::none(),
            ),
            crate::shared::primitives::LayoutSurface::Popup,
        )
        .with_options(options.clone())];

        app.handle_container_layouts(
            ModuleOnMonitor {
                module: parent,
                monitor: &MonitorId::new("DP-1"),
            },
            &layouts,
        )
        .await;

        let requests = std::mem::take(&mut *requests.lock().unwrap());
        let [site] = requests.as_slice() else {
            panic!("expected one spawn, got {}", requests.len());
        };
        assert_eq!(
            site,
            &crate::features::module_runtime::ports::LazySite::new(parent, key)
                .with_options(options)
                .with_surface(crate::shared::primitives::LayoutSurface::Popup)
        );
    });
}

/// Bug: `would_cycle` compared ancestors by bare `ModuleName`, so two
/// sites embedding the same module name with distinct `instance_id`s
/// (e.g. two `ui.module("clock", {instance_id=...})` calls at
/// different popup nesting depths) were incorrectly rejected as a
/// cycle against each other. It must compare the full `ModuleKey`
/// (name + instance id) instead — mirroring how popup-nesting cycle
/// detection elsewhere (`popup::chain::ancestors`) compares full target
/// identity.
#[test]
fn test_would_cycle_distinguishes_instances_from_real_ancestors() {
    crate::test_utils::block_on(async {
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config));
        let (display_tx, display_rx) = mpsc::channel(32);
        let (layout_tx, layout_rx) = mpsc::channel(32);
        let (ui_tx, ui_rx) = mpsc::channel(32);
        let (_system_tx, system_rx) = mpsc::channel(32);
        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManager);

        // `inner` is nested under `outer`; `outer` embeds "clock" as
        // instance "a".
        let outer = ModuleId::new(10);
        let inner = ModuleId::new(20);

        let mut keys = HashMap::new();
        keys.insert(
            outer,
            crate::shared::primitives::ModuleKey::new(
                crate::shared::primitives::ModuleName::new("clock"),
                Some(crate::shared::primitives::ModuleInstanceId::new("a")),
            ),
        );
        keys.insert(
            inner,
            crate::shared::primitives::ModuleKey::from_name(
                crate::shared::primitives::ModuleName::new("calendar"),
            ),
        );

        let mut fake_registry = FakeRegistry::default();
        fake_registry.module_keys = keys;
        fake_registry.parent_of = Box::new(move |id| if id == inner { Some(outer) } else { None });

        let canvas_factory =
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new();
        let deps = ModuleRuntimeDependencies::new(
            ModuleHandles::new(
                hub,
                surface_manager,
                ModuleSenders::new(Arc::new(layout_tx), Arc::new(display_tx), Arc::new(ui_tx)),
            ),
            canvas_factory,
        );
        let channels = StateChannels::new(
            ModuleReceivers::new(layout_rx, display_rx, ui_rx),
            system_rx,
        );
        let app = CrankyApp::new(deps, channels, Box::new(fake_registry)).unwrap();

        let clock_b = crate::shared::primitives::ModuleKey::new(
            crate::shared::primitives::ModuleName::new("clock"),
            Some(crate::shared::primitives::ModuleInstanceId::new("b")),
        );
        let clock_a = crate::shared::primitives::ModuleKey::new(
            crate::shared::primitives::ModuleName::new("clock"),
            Some(crate::shared::primitives::ModuleInstanceId::new("a")),
        );

        // A different instance of "clock" embedded under an ancestor that
        // also embeds "clock" — but as a different instance — is not a
        // cycle.
        assert!(
            !app.would_cycle(inner, &clock_b),
            "distinct instance_ids of the same module name must not be treated as a cycle"
        );

        // The exact same (name, instance_id) reappearing up the real
        // ancestor chain is still correctly rejected as a cycle.
        assert!(
            app.would_cycle(inner, &clock_a),
            "a real ancestor sharing the full ModuleKey must still be caught as a cycle"
        );
    });
}

/// A monitor Hyprland no longer reports must lose its per-monitor state
/// everywhere it's kept — `read_model.module_sizes` /
/// `computed_layouts`, and the shared `module_sizes` signal — while an
/// unrelated live monitor's state is untouched. Without this, the
/// module stays "discovered" forever (`discover_monitors` unions this
/// state with Hyprland's live list) and keeps rendering for a monitor
/// with no surface left.
#[test]
fn test_prune_removed_monitors_drops_only_the_dead_monitor() {
    crate::test_utils::block_on(async {
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config));
        let (display_tx, display_rx) = mpsc::channel(32);
        let (layout_tx, layout_rx) = mpsc::channel(32);
        let (ui_tx, ui_rx) = mpsc::channel(32);
        let (_system_tx, system_rx) = mpsc::channel(32);

        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManager);

        let fake_registry = FakeRegistry::default();

        let canvas_factory =
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new();

        let deps = ModuleRuntimeDependencies::new(
            ModuleHandles::new(
                hub,
                surface_manager,
                ModuleSenders::new(Arc::new(layout_tx), Arc::new(display_tx), Arc::new(ui_tx)),
            ),
            canvas_factory,
        );
        let channels = StateChannels::new(
            ModuleReceivers::new(layout_rx, display_rx, ui_rx),
            system_rx,
        );
        let mut app = CrankyApp::new(deps, channels, Box::new(fake_registry)).unwrap();

        let live_mon = MonitorId::new("eDP-1");
        let dead_mon = MonitorId::new("HDMI-A-1");

        app.read_model
            .module_sizes
            .insert(dead_mon.clone(), HashMap::new());
        app.read_model
            .module_sizes
            .insert(live_mon.clone(), HashMap::new());
        app.read_model
            .computed_layouts
            .insert(dead_mon.clone(), HashMap::new());
        app.read_model
            .computed_layouts
            .insert(live_mon.clone(), HashMap::new());

        let mut shared_sizes = app.hub().module_sizes_rx().borrow().clone();
        shared_sizes.insert(
            dead_mon.clone(),
            crate::shared::primitives::layout::ChildSizesMap::new(),
        );
        shared_sizes.insert(
            live_mon.clone(),
            crate::shared::primitives::layout::ChildSizesMap::new(),
        );
        app.hub().module_sizes_tx().send(shared_sizes).unwrap();

        // Only "eDP-1" is still live per Wayland.
        let mut scales = app.hub().monitor_scales_rx().borrow().clone();
        scales.insert(
            live_mon.clone(),
            crate::shared::primitives::geometry::Scale::new(1.0),
        );
        app.hub().monitor_scales_tx().send(scales).unwrap();

        app.prune_removed_monitors();

        assert!(!app.read_model.module_sizes.contains_key(&dead_mon));
        assert!(app.read_model.module_sizes.contains_key(&live_mon));
        assert!(!app.read_model.computed_layouts.contains_key(&dead_mon));
        assert!(app.read_model.computed_layouts.contains_key(&live_mon));

        let after = app.hub().module_sizes_rx().borrow().clone();
        assert!(!after.contains_key(&dead_mon));
        assert!(after.contains_key(&live_mon));
    });
}

/// The read model starts from the configuration the app is built with.
#[test]
fn test_new_app_reads_the_configuration_it_starts_with() {
    crate::test_utils::block_on(async {
        let (app, _hub) = app_over(config_sampling_every_2500_ms(), FakeRegistry::default());

        assert_eq!(
            app.read_model.config.metrics().update_interval_ms().value(),
            2500
        );
    });
}

/// A size reported by a module's actor reaches the read model through the layout event loop,
/// and the hub with the width the module settled at, and the loop then renders.
#[test]
fn test_handle_layout_events_records_a_reported_size() {
    crate::test_utils::block_on(async {
        let clock = ModuleId::new(3);
        let name = crate::shared::primitives::ModuleName::new("clock");
        let mut registry = FakeRegistry::default();
        registry.module_keys = HashMap::from([(
            clock,
            crate::shared::primitives::ModuleKey::from_name(name.clone()),
        )]);
        let (mut app, hub) = app_over(Config::default(), registry);
        let mut display = FakeDisplayServer::failing("unused");
        let renders = display.renders.clone();

        app.handle_layout_events(
            LayoutEvent::ModuleSizeChanged {
                monitor_id: MonitorId::new("DP-1"),
                module_id: clock,
                size: Size::new(120, 30),
                settled_width: Some(96),
            },
            &mut display,
        )
        .await;

        assert_eq!(
            app.read_model
                .module_sizes
                .get(&MonitorId::new("DP-1"))
                .and_then(|sizes| sizes.get(&clock)),
            Some(&Size::new(120, 30))
        );
        assert_eq!(
            hub.module_sizes_rx()
                .borrow()
                .get(&MonitorId::new("DP-1"))
                .and_then(|sizes| sizes.report_by_name_or_key(&name, None)),
            Some(&ChildReport::new(Size::new(120, 30), Some(96)))
        );
        assert_eq!(renders.load(Ordering::SeqCst), 1);
    });
}

/// A new size is kept per monitor in the read model and reported on the hub under the
/// module's key, together with the width the module settled at.
#[test]
fn test_handle_size_changed_records_the_size_and_reports_the_settled_width() {
    crate::test_utils::block_on(async {
        let clock = ModuleId::new(1);
        let name = crate::shared::primitives::ModuleName::new("clock");
        let mut registry = FakeRegistry::default();
        registry.module_keys = HashMap::from([(
            clock,
            crate::shared::primitives::ModuleKey::from_name(name.clone()),
        )]);
        let (mut app, hub) = app_over(Config::default(), registry);
        let monitor = MonitorId::new("DP-2");

        app.handle_size_changed(
            ModuleOnMonitor {
                module: clock,
                monitor: &monitor,
            },
            ChildReport::new(Size::new(80, 24), Some(64)),
        );

        assert_eq!(
            app.read_model
                .module_sizes
                .get(&monitor)
                .and_then(|sizes| sizes.get(&clock)),
            Some(&Size::new(80, 24))
        );
        let reported = hub.module_sizes_rx().borrow().clone();
        assert_eq!(
            reported
                .get(&monitor)
                .and_then(|sizes| sizes.report_by_name_or_key(&name, None)),
            Some(&ChildReport::new(Size::new(80, 24), Some(64)))
        );
        assert!(!reported.contains_key(&MonitorId::new("DP-1")));
    });
}

/// A Hyprland update re-renders only when the focused monitor moved, and remembers the
/// monitor it moved to.
#[test]
fn test_handle_hyprland_changed_renders_only_when_the_focus_moves() {
    crate::test_utils::block_on(async {
        let (mut app, _hub) = app_over(Config::default(), FakeRegistry::default());
        let mut display = FakeDisplayServer::failing("unused");
        let renders = display.renders.clone();
        let focused_on = |name: &str| {
            crate::shared::events::signals::HyprlandState::new(
                std::collections::BTreeMap::new(),
                std::collections::BTreeMap::new(),
                Some(crate::features::workspaces::domain::MonitorName::new(name)),
            )
        };
        app.handle_hyprland_changed(&focused_on("DP-1"), &mut display);
        assert_eq!(app.focused_monitor, "DP-1");
        assert_eq!(renders.load(Ordering::SeqCst), 1);

        app.handle_hyprland_changed(&focused_on("DP-1"), &mut display);
        assert_eq!(renders.load(Ordering::SeqCst), 1, "the focus did not move");

        app.handle_hyprland_changed(&focused_on("HDMI-A-1"), &mut display);
        assert_eq!(app.focused_monitor, "HDMI-A-1");
        assert_eq!(renders.load(Ordering::SeqCst), 2);
    });
}

/// A lazy spawn that succeeds takes the new actor's layout sender and refreshes the module
/// list of the read model from the registry, recording no error for the site.
#[test]
fn test_ensure_lazy_spawn_keeps_the_spawned_actor() {
    crate::test_utils::block_on(async {
        let parent = ModuleId::new(1);
        let spawned = ModuleId::new(7);
        let mut registry = FakeRegistry::default();
        registry.module_ids = vec![parent];
        registry.spawned_id = Some(spawned);
        let (mut app, hub) = app_over(Config::default(), registry);
        let key = crate::shared::primitives::ModuleKey::from_name(
            crate::shared::primitives::ModuleName::new("calendar"),
        );
        let layouts = [crate::shared::primitives::ChildModuleLayout::new(
            key,
            crate::shared::primitives::ChildBounds::new(
                Rect::new(Position::new(0, 0), Size::new(50, 20)),
                crate::shared::primitives::SizeConstraint::none(),
            ),
            crate::shared::primitives::LayoutSurface::Bar,
        )];

        app.handle_container_layouts(
            ModuleOnMonitor {
                module: parent,
                monitor: &MonitorId::new("DP-1"),
            },
            &layouts,
        )
        .await;

        assert!(app.layout_senders.contains_key(&spawned));
        assert!(!app.layout_senders.contains_key(&parent));
        assert_eq!(app.read_model.module_ids, vec![parent, spawned]);
        assert!(hub.module_errors_rx().borrow().is_empty());
    });
}
