//! How the app spawns, on first sight, a module a parent embeds and no configuration names.

use super::fixtures::{TestApp, app_over, key, laid_out, module_on};
use crate::app::adapter_supervisor::AdapterSupervisor;
use crate::app::test_fakes::FakeRegistry;
use crate::features::module_runtime::ports::LazySite;
use crate::features::module_runtime::test_support::capture_logs;
use crate::shared::config::domain::Config;
use crate::shared::events::signals::SignalKind;
use crate::shared::primitives::{
    ChildModuleLayout, DynamicValue, LayoutSurface, ModuleId, ModuleInstanceId, ModuleKey,
    ModuleName, ModuleOptions, ModuleSite, MonitorId,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

/// A successful lazy spawn takes the registry's new module names, ids by name and keys into the
/// read model, and is logged with the module, its parent and its new id.
#[test]
fn test_ensure_lazy_spawn_refreshes_the_module_lists_of_the_read_model() {
    let logs = capture_logs(|| {
        crate::test_utils::block_on(async {
            let mut registry = FakeRegistry::default();
            registry.spawned_id = Some(ModuleId::new(7));
            let (mut app, _hub) = app_over(Config::default(), registry);
            let calendar = ModuleName::new("calendar");
            app.registry.module_names = HashMap::from([(ModuleId::new(7), calendar.clone())]);
            app.registry.name_to_ids = HashMap::from([(calendar.clone(), vec![ModuleId::new(7)])]);
            app.registry.module_keys =
                HashMap::from([(ModuleId::new(7), ModuleKey::from_name(calendar))]);

            let site = laid_out(key("calendar"), 0, LayoutSurface::Bar);
            app.handle_container_layouts(
                module_on(ModuleId::new(1), &MonitorId::new("DP-1")),
                &[site],
            )
            .await;

            assert_eq!(app.read_model.module_names, app.registry.module_names);
            assert_eq!(app.read_model.name_to_ids, app.registry.name_to_ids);
            assert_eq!(app.read_model.module_keys, app.registry.module_keys);
        });
    });

    assert_eq!(
        logs.lines().collect::<Vec<_>>(),
        [" INFO cranky::app::state: Lazily spawned module module=calendar parent=1 id=7"]
    );
}

/// A lazy spawn that fails is logged as a warning with the registry's error.
#[test]
fn test_ensure_lazy_spawn_logs_a_failed_spawn() {
    let logs = capture_logs(|| {
        crate::test_utils::block_on(async {
            let (mut app, _hub) = app_over(Config::default(), FakeRegistry::default());
            let site = laid_out(key("calendarr"), 0, LayoutSurface::Bar);
            app.handle_container_layouts(
                module_on(ModuleId::new(1), &MonitorId::new("DP-1")),
                &[site],
            )
            .await;
        });
    });

    assert_eq!(
        logs.lines().collect::<Vec<_>>(),
        [
            " WARN cranky::app::state: Lazy module spawn failed module=calendarr parent=1 err=Module not found: calendarr"
        ]
    );
}

/// Once a lazy spawn succeeds, the adapter behind every signal the modules subscribe to is
/// started: here the time adapter, which then feeds the hub's time signal.
#[test]
fn test_ensure_lazy_spawn_starts_the_adapters_the_modules_subscribe_to() {
    crate::test_utils::block_on(async {
        let mut registry = FakeRegistry::default();
        registry.spawned_id = Some(ModuleId::new(7));
        registry.subscribe_to(SignalKind::Time);
        let (mut app, hub) = app_over(Config::default(), registry);
        let home = crate::test_utils::TestHome::new("state-lazy-adapters");
        app.set_adapter_supervisor(AdapterSupervisor::new(
            hub.clone(),
            home.env(),
            Arc::new(crate::shared::dbus::ports::DisconnectedBus),
        ));
        let mut time_rx = hub.time_rx();

        let site = laid_out(key("calendar"), 0, LayoutSurface::Bar);
        app.handle_container_layouts(
            module_on(ModuleId::new(1), &MonitorId::new("DP-1")),
            &[site],
        )
        .await;

        let ticked = tokio::time::timeout(Duration::from_secs(3), time_rx.changed()).await;
        assert!(
            matches!(ticked, Ok(Ok(()))),
            "the time adapter never ticked"
        );
    });
}

/// The site `parent` embeds the module called `name` at, with no instance id.
fn site_of(parent: ModuleId, name: &str) -> ModuleSite {
    ModuleSite::new(Some(parent), key(name))
}

/// Has `parent` lay out `child`, alone, on `DP-1`.
async fn lay_out_one(app: &mut TestApp, parent: ModuleId, child: ChildModuleLayout) {
    let monitor = MonitorId::new("DP-1");
    app.handle_container_layouts(module_on(parent, &monitor), &[child])
        .await;
}

#[test]
fn test_ensure_lazy_spawn_rejects_self_cycle() {
    crate::test_utils::block_on(async {
        // A parent named "calendar" embedding another "calendar" would
        // create a cycle — must be rejected and recorded without ever
        // calling `spawn_module`.
        let parent = ModuleId::new(1);
        let mut registry = FakeRegistry::default();
        registry.module_names = HashMap::from([(parent, ModuleName::new("calendar"))]);
        registry.module_keys = HashMap::from([(parent, key("calendar"))]);
        let spawn_attempts = registry.spawn_module_calls.clone();
        let (mut app, hub) = app_over(Config::default(), registry);

        let child = laid_out(key("calendar"), 0, LayoutSurface::Bar);
        lay_out_one(&mut app, parent, child).await;

        let site = site_of(parent, "calendar");
        assert!(hub.module_errors_rx().borrow().contains_key(&site));
        assert_eq!(spawn_attempts.load(Ordering::SeqCst), 0);
        assert!(app.read_model.computed_layouts.is_empty());
    });
}

#[test]
fn test_ensure_lazy_spawn_does_not_retry_after_recorded_failure() {
    crate::test_utils::block_on(async {
        let parent = ModuleId::new(1);
        let mut registry = FakeRegistry::default();
        registry.module_names = HashMap::from([(parent, ModuleName::new("bar"))]);
        let spawn_attempts = registry.spawn_module_calls.clone();
        let (mut app, hub) = app_over(Config::default(), registry);
        let typo = || laid_out(key("calendarr"), 0, LayoutSurface::Bar);

        // First attempt: spawn_module is called and fails, and the failure
        // is recorded.
        lay_out_one(&mut app, parent, typo()).await;
        let site = site_of(parent, "calendarr");
        assert!(hub.module_errors_rx().borrow().contains_key(&site));
        assert_eq!(spawn_attempts.load(Ordering::SeqCst), 1);

        // Second identical attempt: spawn_module must not be called again.
        lay_out_one(&mut app, parent, typo()).await;
        assert_eq!(spawn_attempts.load(Ordering::SeqCst), 1);
    });
}

/// A lazy spawn hands the registry the parent, the full key, the options and the floating
/// surface of the site the parent laid its child out at.
#[test]
fn test_ensure_lazy_spawn_passes_the_site_options_and_surface_to_the_registry() {
    crate::test_utils::block_on(async {
        let parent = ModuleId::new(1);
        let registry = FakeRegistry::default();
        let requests = registry.spawn_requests.clone();
        let (mut app, _hub) = app_over(Config::default(), registry);
        let cal_2 = ModuleKey::new(
            ModuleName::new("calendar"),
            Some(ModuleInstanceId::new("cal-2")),
        );
        let options = ModuleOptions::new(HashMap::from([(
            "format".to_string(),
            DynamicValue::String("%A %d".to_string()),
        )]));
        let child = laid_out(cal_2.clone(), 0, LayoutSurface::Popup).with_options(options.clone());

        lay_out_one(&mut app, parent, child).await;

        let requests = std::mem::take(&mut *requests.lock().unwrap());
        let [site] = requests.as_slice() else {
            panic!("expected one spawn, got {}", requests.len());
        };
        let expected = LazySite::new(parent, cal_2).with_options(options);
        assert_eq!(site, &expected.with_surface(LayoutSurface::Popup));
    });
}

/// The key of the instance `instance` of the module called `name`.
fn instance_key(name: &str, instance: &str) -> ModuleKey {
    ModuleKey::new(ModuleName::new(name), Some(ModuleInstanceId::new(instance)))
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
        // `inner` is nested under `outer`; `outer` embeds "clock" as
        // instance "a".
        let outer = ModuleId::new(10);
        let inner = ModuleId::new(20);
        let mut registry = FakeRegistry::default();
        registry.module_keys = HashMap::from([
            (outer, instance_key("clock", "a")),
            (inner, key("calendar")),
        ]);
        registry.parent_of = Box::new(move |id| (id == inner).then_some(outer));
        let (app, _hub) = app_over(Config::default(), registry);

        // A different instance of "clock" embedded under an ancestor that
        // also embeds "clock" — but as a different instance — is not a
        // cycle.
        assert!(
            !app.would_cycle(inner, &instance_key("clock", "b")),
            "distinct instance_ids of the same module name must not be treated as a cycle"
        );

        // The exact same (name, instance_id) reappearing up the real
        // ancestor chain is still correctly rejected as a cycle.
        assert!(
            app.would_cycle(inner, &instance_key("clock", "a")),
            "a real ancestor sharing the full ModuleKey must still be caught as a cycle"
        );
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

        let child = laid_out(key("calendar"), 0, LayoutSurface::Bar);
        lay_out_one(&mut app, parent, child).await;

        assert!(app.layout_senders.contains_key(&spawned));
        assert!(!app.layout_senders.contains_key(&parent));
        assert_eq!(app.read_model.module_ids, vec![parent, spawned]);
        assert!(hub.module_errors_rx().borrow().is_empty());
    });
}
