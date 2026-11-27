//! How the app records where parents lay their children out, the sizes modules report, and
//! the monitors that come and go.

use super::fakes::RecordingDisplay;
use super::fixtures::{
    TestApp, app_over, bar_embedding_clock, bus_with, clock_laid_out_on, harness, key, laid_out,
    layout_channel, module_on, send_all,
};
use crate::app::test_fakes::{FakeDisplayServer, FakeRegistry, FakeSni};
use crate::features::module_runtime::ports::LayoutEvent;
use crate::features::module_runtime::test_support::capture_logs;
use crate::shared::config::domain::Config;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::geometry::{Scale, Size};
use crate::shared::primitives::layout::ChildSizesMap;
use crate::shared::primitives::{
    ChildReport, LayoutSurface, ModuleId, ModuleKey, ModuleName, MonitorId,
};
use std::collections::HashMap;
use std::sync::atomic::Ordering;

/// A registry where module 1 embeds module 2 at every key.
fn parent_embedding_child() -> FakeRegistry {
    let mut registry = FakeRegistry::default();
    registry.resolve_site =
        Box::new(|parent, _| (parent == Some(ModuleId::new(1))).then(|| ModuleId::new(2)));
    registry.parent_of = Box::new(|id| (id == ModuleId::new(2)).then(|| ModuleId::new(1)));
    registry
}

/// Placing a child and withdrawing its site are each traced with the child and the monitor.
#[test]
fn test_handle_container_layouts_traces_each_placed_and_withdrawn_child() {
    let logs = capture_logs(|| {
        crate::test_utils::block_on(async {
            let (mut app, _hub) = app_over(Config::default(), parent_embedding_child());
            let monitor = MonitorId::new("DP-1");
            let child = laid_out(key("calendar"), 30, LayoutSurface::Popup);

            app.handle_container_layouts(module_on(ModuleId::new(1), &monitor), &[child])
                .await;
            app.handle_container_layouts(module_on(ModuleId::new(1), &monitor), &[])
                .await;
        });
    });

    assert_eq!(
        logs.lines().collect::<Vec<_>>(),
        [
            "TRACE cranky::app::state: Updating computed_layouts for child module child=2 monitor=DP-1 bounds=Rect { position: Position { x: 30, y: 0 }, size: Size { width: 50, height: 20 } }",
            "TRACE cranky::app::state: Embedding site withdrawn; suspending child on this monitor child=2 monitor=DP-1",
        ]
    );
}

/// A child laid out on two monitors is sent its bounds on both, each where its parent put it.
#[test]
fn test_container_layouts_calculated_preserves_multi_monitors() {
    crate::test_utils::block_on(async {
        let (bar_sender, _bar_layouts) = layout_channel();
        let (clock_sender, clock_layouts) = layout_channel();
        let mut registry = bar_embedding_clock();
        registry.senders = HashMap::from([
            (ModuleId::new(0), bar_sender),
            (ModuleId::new(1), clock_sender),
        ]);
        let mut harness = harness(Config::default(), registry);
        let events = [
            clock_laid_out_on("DP-1", 100),
            clock_laid_out_on("DP-2", 150),
        ];
        send_all(&harness.queues.layout, events).await;

        let bus = bus_with(&harness.hub, FakeSni);
        let _ = harness.app.run(RecordingDisplay::new(), bus).await;

        let current = clock_layouts.borrow().clone();
        assert_eq!(current[&MonitorId::new("DP-1")].rect().x(), 100);
        assert_eq!(current[&MonitorId::new("DP-2")].rect().x(), 150);
    });
}

/// Which site each parent embeds `calendar` at: parent 10 at site 11, parent 20 at site 21.
const CALENDAR_SITES: [(u32, u32); 2] = [(10, 11), (20, 21)];

/// Two different parents (10 and 20) each embedding a module named `calendar`.
fn calendar_under_two_parents() -> FakeRegistry {
    let mut registry = FakeRegistry::default();
    registry.resolve_site = Box::new(|parent, key| {
        let owner = parent.filter(|_| key.name().as_str() == "calendar")?;
        let sites = CALENDAR_SITES.iter();
        let found = sites
            .into_iter()
            .find(|(id, _)| ModuleId::new(*id) == owner);
        found.map(|(_, site)| ModuleId::new(*site))
    });
    // Each site's own parent — consulted when the *other* parent's
    // render touches the same monitor, to confirm `site_a`/`site_b`
    // belong to a different parent and so must not be suspended.
    registry.parent_of = Box::new(|child| {
        let found = CALENDAR_SITES
            .iter()
            .find(|(_, site)| ModuleId::new(*site) == child);
        found.map(|(owner, _)| ModuleId::new(*owner))
    });
    registry
}

#[test]
fn test_handle_container_layouts_does_not_cross_talk_between_sites() {
    crate::test_utils::block_on(async {
        // Two different parents (10 and 20) each embed a module named
        // "calendar" — resolve_site must route each parent's layout event to
        // its own site's ModuleId, never the other parent's.
        let (mut app, _hub) = app_over(Config::default(), calendar_under_two_parents());
        let monitor = MonitorId::new("DP-1");
        let (site_a, site_b) = (ModuleId::new(11), ModuleId::new(21));

        let at_10 = laid_out(key("calendar"), 10, LayoutSurface::Bar);
        app.handle_container_layouts(module_on(ModuleId::new(10), &monitor), &[at_10])
            .await;
        let at_90 = laid_out(key("calendar"), 90, LayoutSurface::Bar);
        app.handle_container_layouts(module_on(ModuleId::new(20), &monitor), &[at_90])
            .await;

        let layouts = &app.read_model.computed_layouts[&monitor];
        assert_eq!(layouts[&site_a].rect().x(), 10);
        assert_eq!(layouts[&site_b].rect().x(), 90);
        assert_eq!(layouts.len(), 2);
    });
}

/// Whether the read model holds a rect for `child` on `monitor`.
fn has_rect(app: &TestApp, monitor: &MonitorId, child: ModuleId) -> bool {
    app.read_model
        .computed_layouts
        .get(monitor)
        .is_some_and(|monitor_layouts| monitor_layouts.contains_key(&child))
}

#[test]
fn test_handle_container_layouts_suspends_child_whose_site_closed() {
    crate::test_utils::block_on(async {
        // Decision 14: once a parent's render no longer embeds a child on a
        // monitor (its popup closed), the stale `computed_layouts` entry
        // must be cleared and the child told directly — this is what stops
        // it from rendering there again (gap 8).
        let (mut app, _hub) = app_over(Config::default(), parent_embedding_child());
        let child = ModuleId::new(2);
        let (child_sender, child_layout_rx) = layout_channel();
        app.layout_senders.insert(child, child_sender);
        let monitor = MonitorId::new("DP-1");
        let parent = module_on(ModuleId::new(1), &monitor);

        // Popup open: the child gets a rect on DP-1.
        let popup = laid_out(key("calendar"), 0, LayoutSurface::Popup);
        app.handle_container_layouts(parent, &[popup]).await;
        assert!(has_rect(&app, &monitor, child));
        assert!(child_layout_rx.borrow().contains_key(&monitor));

        // Popup closed: the parent's next render carries no children at all.
        app.handle_container_layouts(parent, &[]).await;
        let message = "the stale entry must be cleared once the site closes";
        assert!(!has_rect(&app, &monitor, child), "{message}");
        assert!(
            !child_layout_rx.borrow().contains_key(&monitor),
            "the child must be told its monitor is gone, not left with a stale rect"
        );
    });
}

/// A registry that knows `clock` as module `id`, by its key.
fn keyed_clock(id: ModuleId) -> FakeRegistry {
    let mut registry = FakeRegistry::default();
    registry.module_keys = HashMap::from([(id, ModuleKey::from_name(ModuleName::new("clock")))]);
    registry
}

/// What the hub's size signal reports for `clock` on `monitor`.
fn reported_clock(hub: &SignalHub, monitor: &str) -> Option<ChildReport> {
    let sizes = hub.module_sizes_rx().borrow().clone();
    let on_monitor = sizes.get(&MonitorId::new(monitor))?;
    on_monitor
        .report_by_name_or_key(&ModuleName::new("clock"), None)
        .copied()
}

/// What the read model keeps as the size of `module` on `monitor`.
fn kept_size(app: &TestApp, monitor: &str, module: ModuleId) -> Option<Size> {
    let sizes = app.read_model.module_sizes.get(&MonitorId::new(monitor))?;
    sizes.get(&module).copied()
}

/// A size reported by a module's actor reaches the read model through the layout event loop,
/// and the hub with the width the module settled at, and the loop then renders.
#[test]
fn test_handle_layout_events_records_a_reported_size() {
    crate::test_utils::block_on(async {
        let clock = ModuleId::new(3);
        let (mut app, hub) = app_over(Config::default(), keyed_clock(clock));
        let mut display = FakeDisplayServer::failing("unused");
        let renders = display.renders.clone();
        let size_change = LayoutEvent::ModuleSizeChanged {
            monitor_id: MonitorId::new("DP-1"),
            module_id: clock,
            size: Size::new(120, 30),
            settled_width: Some(96),
        };

        app.handle_layout_events(size_change, &mut display).await;

        assert_eq!(kept_size(&app, "DP-1", clock), Some(Size::new(120, 30)));
        assert_eq!(
            reported_clock(&hub, "DP-1"),
            Some(ChildReport::new(Size::new(120, 30), Some(96)))
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
        let (mut app, hub) = app_over(Config::default(), keyed_clock(clock));
        let monitor = MonitorId::new("DP-2");

        let report = ChildReport::new(Size::new(80, 24), Some(64));
        app.handle_size_changed(module_on(clock, &monitor), report);

        assert_eq!(kept_size(&app, "DP-2", clock), Some(Size::new(80, 24)));
        assert_eq!(
            reported_clock(&hub, "DP-2"),
            Some(ChildReport::new(Size::new(80, 24), Some(64)))
        );
        let reported = hub.module_sizes_rx().borrow().clone();
        assert!(!reported.contains_key(&MonitorId::new("DP-1")));
    });
}

/// Gives `monitor` an empty entry everywhere per-monitor state is kept: the read model's
/// sizes and layouts, and the hub's size signal.
fn seed_monitor(app: &mut TestApp, monitor: &MonitorId) {
    let model = &mut app.read_model;
    model.module_sizes.insert(monitor.clone(), HashMap::new());
    model
        .computed_layouts
        .insert(monitor.clone(), HashMap::new());
    let mut shared_sizes = app.hub().module_sizes_rx().borrow().clone();
    shared_sizes.insert(monitor.clone(), ChildSizesMap::new());
    app.hub().module_sizes_tx().send(shared_sizes).unwrap();
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
        let (mut app, hub) = app_over(Config::default(), FakeRegistry::default());
        let live_mon = MonitorId::new("eDP-1");
        let dead_mon = MonitorId::new("HDMI-A-1");
        seed_monitor(&mut app, &dead_mon);
        seed_monitor(&mut app, &live_mon);

        // Only "eDP-1" is still live per Wayland.
        let mut scales = hub.monitor_scales_rx().borrow().clone();
        scales.insert(live_mon.clone(), Scale::new(1.0));
        hub.monitor_scales_tx().send(scales).unwrap();

        app.prune_removed_monitors();

        assert!(!app.read_model.module_sizes.contains_key(&dead_mon));
        assert!(app.read_model.module_sizes.contains_key(&live_mon));
        assert!(!app.read_model.computed_layouts.contains_key(&dead_mon));
        assert!(app.read_model.computed_layouts.contains_key(&live_mon));
        let after = hub.module_sizes_rx().borrow().clone();
        assert!(!after.contains_key(&dead_mon));
        assert!(after.contains_key(&live_mon));
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
