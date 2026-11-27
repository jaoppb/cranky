//! How the app reloads modules: for a changed script or stylesheet, and for a new
//! configuration.

use super::fakes::CallLog;
use super::fakes::RecordingDisplay;
use super::fixtures::{
    TestApp, config_sampling_every_2500_ms, drain, errors_in, harness, layout_channel, send_all,
};
use crate::app::commands::SystemCommand;
use crate::app::registry::WatchLayoutSender;
use crate::app::test_fakes::FakeRegistry;
use crate::features::module_runtime::test_support::capture_logs;
use crate::features::styling::domain::StyleSheetName;
use crate::shared::config::domain::Config;
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::{ModuleId, ModuleKey, ModuleName, MonitorId};
use std::collections::{HashMap, HashSet};

/// A registry of a `bar` (id 0) and a `clock` (id 1), whose stylesheet `clock-theme` the clock
/// uses and `bar-theme` the bar uses.
fn bar_and_clock() -> FakeRegistry {
    let mut registry = FakeRegistry::default();
    let bar = ModuleName::new("bar");
    let clock = ModuleName::new("clock");
    registry.module_names = HashMap::from([
        (ModuleId::new(0), bar.clone()),
        (ModuleId::new(1), clock.clone()),
    ]);
    registry.name_to_ids = HashMap::from([
        (bar.clone(), vec![ModuleId::new(0)]),
        (clock.clone(), vec![ModuleId::new(1)]),
    ]);
    registry.style_users = HashMap::from([
        ("clock-theme".to_string(), vec![clock]),
        ("bar-theme".to_string(), vec![bar]),
        ("shared".to_string(), Vec::new()),
    ]);
    registry
}

fn sheet(name: &str) -> StyleSheetName {
    StyleSheetName::new(name).unwrap()
}

/// The ids the app holds a layout sender for, in order.
fn sender_ids(senders: &HashMap<ModuleId, WatchLayoutSender>) -> Vec<ModuleId> {
    let mut ids: Vec<_> = senders.keys().copied().collect();
    ids.sort_by_key(ToString::to_string);
    ids
}

/// What reloading the stylesheet `name` over `registry` does: whether a render is then due,
/// the registry calls it made, and the ids the app then holds a layout sender for.
fn reload_style_over(registry: FakeRegistry, name: &str) -> (bool, Vec<String>, Vec<ModuleId>) {
    crate::test_utils::block_on(async {
        let calls = registry.calls.clone();
        let mut app = harness(Config::default(), registry).app;
        drain(&calls);
        let deps = app.deps.clone();
        let due = app.handle_reload_style(&sheet(name), &deps);
        (due, drain(&calls), sender_ids(&app.layout_senders))
    })
}

/// A change to the base stylesheet reloads every module, keeps the senders of those that
/// reloaded, says a render is due even when a reload failed, and logs no failure.
#[test]
fn test_reload_of_the_base_sheet_reloads_every_module() {
    let mut registry = bar_and_clock();
    registry.failing_reloads = HashSet::from([ModuleName::new("clock")]);
    let mut outcome = None;
    let logs = capture_logs(|| outcome = Some(reload_style_over(registry, "base")));
    let (due, mut reloads, reloaded_ids) = outcome.unwrap();

    assert!(due);
    reloads.sort();
    assert_eq!(reloads, ["reload bar", "reload clock"]);
    assert_eq!(reloaded_ids, [ModuleId::new(0)]);
    assert_eq!(
        logs.lines().collect::<Vec<_>>(),
        [
            " INFO cranky::app::state: Reloading style: base",
            "DEBUG cranky::app::state: Base stylesheet changed; reloading all active modules",
        ]
    );
}

/// A change to another stylesheet reloads only the modules that use it, and keeps their senders.
#[test]
fn test_reload_of_a_sheet_reloads_only_the_modules_using_it() {
    let mut outcome = None;
    let logs = capture_logs(|| outcome = Some(reload_style_over(bar_and_clock(), "clock-theme")));
    let (due, reloads, reloaded_ids) = outcome.unwrap();

    assert_eq!(reloads, ["reload clock"]);
    assert_eq!(reloaded_ids, [ModuleId::new(1)]);
    assert!(due);
    assert_eq!(
        logs.lines().collect::<Vec<_>>(),
        [
            " INFO cranky::app::state: Reloading style: clock-theme",
            "DEBUG cranky::app::state: Reloading modules dependent on modified stylesheet stylesheet=clock-theme dependent_modules=[\"clock\"]",
        ]
    );
}

/// A stylesheet change asks for a render only when one of its modules reloaded; each failed
/// reload is logged.
#[test]
fn test_reload_of_a_sheet_is_due_only_when_a_module_reloaded() {
    let logs = capture_logs(|| {
        crate::test_utils::block_on(async {
            let mut registry = bar_and_clock();
            registry.failing_reloads = HashSet::from([ModuleName::new("clock")]);
            let mut app = harness(Config::default(), registry).app;
            let deps = app.deps.clone();

            assert!(!app.handle_reload_style(&sheet("clock-theme"), &deps));
            assert!(!app.handle_reload_style(&sheet("shared"), &deps));
            assert!(app.handle_reload_style(&sheet("bar-theme"), &deps));
            assert_eq!(sender_ids(&app.layout_senders), [ModuleId::new(0)]);
        });
    });

    assert_eq!(
        errors_in(&logs),
        ["ERROR cranky::app::state: Failed to reload module clock: Module not found: clock"]
    );
}

/// Every system command already queued behind the first one is carried out, in the order it
/// was sent, and the display renders once at the end because a reload succeeded.
#[test]
fn test_handle_system_commands_drains_the_queue_and_renders_once() {
    crate::test_utils::block_on(async {
        let registry = bar_and_clock();
        let calls = registry.calls.clone();
        let mut harness = harness(Config::default(), registry);
        drain(&calls);
        let mut display = RecordingDisplay::new();
        let renders = display.calls();
        let queued = [
            SystemCommand::ReloadStyle(sheet("clock-theme")),
            SystemCommand::ReloadModule(ModuleName::new("bar")),
        ];
        send_all(&harness.queues.system, queued).await;

        let reload_clock = SystemCommand::ReloadModule(ModuleName::new("clock"));
        harness
            .app
            .handle_system_commands(reload_clock, &mut display);

        let reloads = ["reload clock", "reload clock", "reload bar"];
        assert_eq!(drain(&calls), reloads);
        let ids = [ModuleId::new(0), ModuleId::new(1)];
        assert_eq!(sender_ids(&harness.app.layout_senders), ids);
        assert_eq!(drain(&renders), ["render"]);
    });
}

/// A failed module reload is logged and does not render; neither does a stylesheet change no
/// module uses.
#[test]
fn test_handle_system_commands_without_a_reload_does_not_render() {
    let mut renders = Vec::new();
    let logs = capture_logs(|| {
        crate::test_utils::block_on(async {
            let mut registry = bar_and_clock();
            registry.failing_reloads = HashSet::from([ModuleName::new("clock")]);
            let mut harness = harness(Config::default(), registry);
            let mut display = RecordingDisplay::new();
            let queued = [SystemCommand::ReloadStyle(sheet("shared"))];
            send_all(&harness.queues.system, queued).await;

            harness.app.handle_system_commands(
                SystemCommand::ReloadModule(ModuleName::new("clock")),
                &mut display,
            );
            renders = drain(&display.calls());
        });
    });

    assert_eq!(renders, Vec::<String>::new());
    assert!(logs.contains(" INFO cranky::app::state: Reloading module: clock\n"));
    assert!(logs.contains(
        "ERROR cranky::app::state: Failed to reload module clock: Module not found: clock\n"
    ));
}

/// A size kept for `module` on `monitor` in the read model.
fn sizes_with(monitor: &str, module: ModuleId) -> HashMap<MonitorId, HashMap<ModuleId, Size>> {
    HashMap::from([(
        MonitorId::new(monitor),
        HashMap::from([(module, Size::new(10, 10))]),
    )])
}

/// Gives `registry` a different module list from the one the app was built with: a root `panel`
/// (id 5) and the sender `spawn_all` hands out for it.
fn replace_modules(registry: &mut FakeRegistry) {
    let panel = ModuleName::new("panel");
    registry.root_module = Some(ModuleId::new(5));
    registry.module_ids = vec![ModuleId::new(5)];
    registry.module_names = HashMap::from([(ModuleId::new(5), panel.clone())]);
    registry.name_to_ids = HashMap::from([(panel.clone(), vec![ModuleId::new(5)])]);
    registry.module_keys = HashMap::from([(ModuleId::new(5), ModuleKey::from_name(panel))]);
    registry.senders = HashMap::from([(ModuleId::new(5), layout_channel().0)]);
}

/// An app over `registry` that holds a size reported on `DP-1`, whose registry then changes its
/// module list (`replace_modules`), with the handle on the registry's calls from then on.
fn app_with_a_changed_registry(registry: FakeRegistry) -> (TestApp, CallLog) {
    let calls = registry.calls.clone();
    let mut app = harness(Config::default(), registry).app;
    drain(&calls);
    app.read_model.module_sizes = sizes_with("DP-1", ModuleId::new(1));
    replace_modules(&mut app.registry);
    (app, calls)
}

/// A new configuration replaces the old one, forgets every reported size, reloads the registry
/// from it, takes the registry's new module lists and respawns every module.
#[test]
fn test_handle_config_changed_reloads_the_registry_and_respawns_the_modules() {
    let logs = capture_logs(|| {
        crate::test_utils::block_on(async {
            let (mut app, calls) = app_with_a_changed_registry(bar_and_clock());

            app.handle_config_changed(config_sampling_every_2500_ms());

            assert_eq!(drain(&calls), ["clear", "load", "spawn_all"]);
            let model = &app.read_model;
            assert_eq!(model.config.metrics().update_interval_ms().value(), 2500);
            assert!(model.module_sizes.is_empty());
            assert_eq!(model.root_module, Some(ModuleId::new(5)));
            assert_eq!(model.module_ids, [ModuleId::new(5)]);
            assert_eq!(model.module_names, app.registry.module_names);
            assert_eq!(model.name_to_ids, app.registry.name_to_ids);
            assert_eq!(model.module_keys, app.registry.module_keys);
            assert_eq!(sender_ids(&app.layout_senders), [ModuleId::new(5)]);
        });
    });

    assert_eq!(
        logs.lines().collect::<Vec<_>>(),
        [" INFO cranky::app::state: Config hot-reload triggered in App"]
    );
}

/// When the registry fails to load the new configuration, the failure is logged and the app
/// keeps its module lists and senders, though it holds the new configuration and has forgotten
/// every reported size.
#[test]
fn test_handle_config_changed_keeps_the_modules_when_the_registry_fails_to_load() {
    let logs = capture_logs(|| {
        crate::test_utils::block_on(async {
            let mut registry = bar_and_clock();
            registry.root_module = Some(ModuleId::new(0));
            let (mut app, calls) = app_with_a_changed_registry(registry);
            app.layout_senders
                .insert(ModuleId::new(1), layout_channel().0);
            app.registry.load_failure = Some(ModuleName::new("panel"));

            app.handle_config_changed(config_sampling_every_2500_ms());

            assert_eq!(drain(&calls), ["clear", "load"]);
            let model = &app.read_model;
            assert_eq!(model.config.metrics().update_interval_ms().value(), 2500);
            assert!(model.module_sizes.is_empty());
            assert_eq!(model.root_module, Some(ModuleId::new(0)));
            assert_eq!(model.module_ids, Vec::<ModuleId>::new());
            assert_eq!(model.module_names, bar_and_clock().module_names);
            assert_eq!(sender_ids(&app.layout_senders), [ModuleId::new(1)]);
        });
    });

    assert!(logs.contains(
        "ERROR cranky::app::state: Failed to reload registry on config change: Module not found: panel\n"
    ));
}
