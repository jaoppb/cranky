//! Spawning the loaded modules, lazily spawning a site, building a site's context, and
//! clearing the registry.
use super::super::{ModuleSiteInfo, WatchLayoutSender};
use super::{LoadedBar, TestRegistryPort, loaded, test_deps};
use crate::features::module_runtime::ports::{
    LayoutSender, LazySite, ModuleRegistryPort, RegistryLoadError,
};
use crate::shared::config::domain::ModuleConfig;
use crate::shared::primitives::geometry::{Position, Rect, Size};
use crate::shared::primitives::{
    ChildBounds, LayoutSurface, ModuleInstanceId, ModuleKey, ModuleName, ModuleOptions, MonitorId,
    SizeConstraint,
};
use std::collections::HashMap;

const BAR_WITH_CLOCK: &str = r#"
    [root]
    name = "bar"
    left = ["clock"]
"#;

#[test]
fn test_module_error_display() {
    let err2 = crate::shared::scripting::ports::ModuleError::Internal {
        message: "error".into(),
    };
    assert_eq!(err2.to_string(), "Internal module error: error");
}

#[test]
fn test_watch_layout_sender() {
    let (tx, rx) = tokio::sync::watch::channel(HashMap::new());
    let sender = WatchLayoutSender { tx };

    let mut layout = HashMap::new();
    layout.insert(
        MonitorId::new("1"),
        ChildBounds::new(
            Rect::new(Position::new(0, 0), Size::new(0, 0)),
            SizeConstraint::none(),
        ),
    );
    sender.send_layout(layout.clone());

    let current = rx.borrow().clone();
    assert!(current.contains_key(&MonitorId::new("1")));
}

#[test]
fn test_module_registry_clear() {
    let home = crate::test_utils::TestHome::new("registry");
    let (mut registry, _config) = loaded(home.env(), BAR_WITH_CLOCK);
    TestRegistryPort::clear(&mut registry);

    assert_eq!(registry.module_ids, []);
    assert!(registry.modules.is_empty());
}

#[test]
fn test_module_registry_register_dbus() {
    crate::test_utils::block_on(async {
        let home = crate::test_utils::TestHome::new("registry");
        let (registry, config) = loaded(home.env(), BAR_WITH_CLOCK);

        let hub = std::sync::Arc::new(crate::shared::events::signals::SignalHub::new(config));
        let mut dbus_manager =
            crate::shared::dbus::subscription_manager::DbusSubscriptionManager::new(
                std::sync::Arc::new(crate::shared::dbus::ports::DisconnectedBus),
                &hub,
            );
        TestRegistryPort::register_dbus_subscriptions(&registry, &mut dbus_manager).await;
    });
}

#[test]
fn test_module_registry_spawn_all() {
    crate::test_utils::block_on(async {
        let home = crate::test_utils::TestHome::new("registry");
        let (mut registry, config) = loaded(home.env(), BAR_WITH_CLOCK);
        let senders = registry.spawn_all(&test_deps(&config));

        assert_eq!(senders.len(), 2); // bar + clock
        assert!(registry.modules.is_empty());
        assert!(
            TestRegistryPort::active_signal_subscriptions(&registry)
                .contains(&crate::shared::events::signals::SignalKind::Time)
        );
    });
}

#[test]
fn test_spawn_module_lazy_mints_a_working_site() {
    crate::test_utils::block_on(async {
        let mut bar = LoadedBar::new();
        let site = bar
            .site_named("calendar")
            .with_surface(LayoutSurface::Popup);
        let id = bar.spawn(site).unwrap();

        assert_ne!(id, bar.root);
        let key = ModuleKey::from_name(ModuleName::new("calendar"));
        assert_eq!(
            TestRegistryPort::resolve_site(&bar.registry, Some(bar.root), &key),
            Some(id)
        );
        assert_eq!(
            TestRegistryPort::parent_of(&bar.registry, id),
            Some(bar.root)
        );
        // The lazily spawned module was handed off to its own actor task,
        // not left sitting in the pre-spawn pool.
        assert!(!bar.registry.modules.contains_key(&id));
    });
}

#[test]
fn test_spawn_module_lazy_propagates_not_found() {
    let mut bar = LoadedBar::new();
    let result = bar.spawn(bar.site_named("does-not-exist"));
    assert!(matches!(result, Err(RegistryLoadError::ModuleNotFound(_))));
}

/// Regression: `clear()` used to reset `next_module_id` without ever
/// aborting a lazily spawned actor's task. Orphaned that way, the
/// actor (and its subsurface, channels, etc.) kept running forever
/// past the config reload that cleared the registry, and could end up
/// sharing a freshly re-minted `ModuleId` with a brand-new
/// config-declared module once `next_module_id` restarted at 0.
#[test]
fn test_clear_aborts_orphaned_lazy_actors() {
    crate::test_utils::block_on(async {
        let mut bar = LoadedBar::new();
        let site = bar
            .site_named("calendar")
            .with_surface(LayoutSurface::Popup);
        let id = bar.spawn(site).unwrap();
        let registry = &mut bar.registry;

        let abort_handle = registry
            .lazy_actor_handles
            .get(&id)
            .expect("the lazily spawned actor's JoinHandle must be tracked")
            .abort_handle();
        assert!(
            !abort_handle.is_finished(),
            "the actor must still be running right after it's spawned"
        );

        registry.clear();
        assert!(
            registry.lazy_actor_handles.is_empty(),
            "clear() must drop every tracked handle"
        );
        assert_task_stops(&abort_handle).await;
    });
}

/// Gives the aborted task behind `abort_handle` a few turns to stop, then checks it did.
async fn assert_task_stops(abort_handle: &tokio::task::AbortHandle) {
    for _ in 0..10 {
        if abort_handle.is_finished() {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(
        abort_handle.is_finished(),
        "clear() must abort every lazily spawned actor, not just stop tracking it"
    );
}

/// A lazy spawn keeps everything its site asked for: the options the module is
/// initialised with, its full key (instance id included), its parent and its surface.
#[test]
fn test_spawn_module_keeps_the_site_options_key_parent_and_surface() {
    crate::test_utils::block_on(async {
        let mut bar = LoadedBar::new();
        let key = ModuleKey::new(
            ModuleName::new("calendar"),
            Some(ModuleInstanceId::new("cal-2")),
        );
        let options = ModuleOptions::new(HashMap::from([(
            "format".to_string(),
            crate::shared::primitives::DynamicValue::String("%A %d".to_string()),
        )]));
        let site = LazySite::new(bar.root, key.clone())
            .with_options(options.clone())
            .with_surface(LayoutSurface::Panel);

        let id = bar.spawn(site).unwrap();

        let registry = &bar.registry;
        let configured = registry.module_configs.get(&id).map(ModuleConfig::options);
        assert_eq!(configured, Some(&options));
        assert_eq!(TestRegistryPort::module_keys(registry).get(&id), Some(&key));
        assert_eq!(TestRegistryPort::parent_of(registry, id), Some(bar.root));
        let surface = registry.sites.get(&id).map(ModuleSiteInfo::surface);
        assert_eq!(surface, Some(LayoutSurface::Panel));
    });
}

/// The context built for a site's actor carries the site's id, parent and surface, and
/// reads its layout from the channel the site's sender feeds.
#[test]
fn test_build_site_context_wires_the_site_and_its_layout_channel() {
    crate::test_utils::block_on(async {
        let mut bar = LoadedBar::new();
        let site = bar
            .site_named("calendar")
            .with_surface(LayoutSurface::Panel);
        let id = bar.spawn(site).unwrap();

        let (mut ctx, sender) = bar.registry.build_site_context(id, &bar.deps);

        assert_eq!(ctx.id(), id);
        assert_eq!(ctx.parent_id(), Some(bar.root));
        assert_eq!(ctx.surface(), LayoutSurface::Panel);
        let monitor = MonitorId::new("DP-1");
        let bounds = ChildBounds::new(
            Rect::new(Position::new(3, 0), Size::new(40, 20)),
            SizeConstraint::none(),
        );
        sender.send_layout(HashMap::from([(monitor.clone(), bounds)]));
        assert_eq!(ctx.rxs_mut().0.borrow().get(&monitor), Some(&bounds));
    });
}
