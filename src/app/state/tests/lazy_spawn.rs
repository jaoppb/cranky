//! How the app spawns, on first sight, a module a parent embeds and no configuration names.

use super::fixtures::{app_over, key, laid_out, module_on};
use crate::app::adapter_supervisor::AdapterSupervisor;
use crate::app::test_fakes::FakeRegistry;
use crate::features::module_runtime::test_support::capture_logs;
use crate::shared::config::domain::Config;
use crate::shared::events::signals::SignalKind;
use crate::shared::primitives::{LayoutSurface, ModuleId, ModuleKey, ModuleName, MonitorId};
use std::collections::HashMap;
use std::sync::Arc;
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
