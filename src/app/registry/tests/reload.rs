//! Reloading a module: every site configured with its name gets a fresh actor, on the
//! surface it was spawned on.
use super::super::{ModuleRegistry, ModuleSiteInfo};
use super::{
    FAILING_INIT, LoadedBar, PLAIN_MODULE, TestDeps, loaded, test_deps, write_user_module,
};
use crate::features::module_runtime::ports::{
    LazySite, ModuleRegistryPort, ModuleSetup, RegistryLoadError,
};
use crate::features::styling::domain::StyleSheetName;
use crate::shared::config::domain::{Config, EngineId, EngineSelection, ModuleConfig};
use crate::shared::events::signals::SignalKind;
use crate::shared::primitives::{LayoutSurface, ModuleId, ModuleKey, ModuleName, ModuleOptions};

/// Bug A regression: `reload_module` used to rebuild a reloaded
/// module's `ModuleContext` via `.with_parent(parent_id)` alone, never
/// chaining `.with_surface(...)` — so hot-reloading a module embedded
/// via `ui.module()` inside an open popup/panel/tooltip silently reset
/// its `LayoutSurface` back to the `Bar` default, breaking where its
/// Wayland subsurface gets parented. `build_site_context` — now shared
/// by `spawn_all`, `spawn_module_lazy` and `reload_module` — must carry
/// the site's recorded surface forward across a reload instead.
#[test]
fn test_reload_preserves_embedded_surface() {
    crate::test_utils::block_on(async {
        let mut bar = LoadedBar::new();
        // Spawn "calendar" lazily, embedded under the root as a Popup site — mirroring a
        // module embedded via `ui.module()` inside an open `ui.popup`.
        let site = bar
            .site_named("calendar")
            .with_surface(LayoutSurface::Popup);
        let id = bar.spawn(site).unwrap();
        let surface_of =
            |registry: &ModuleRegistry| registry.sites.get(&id).map(ModuleSiteInfo::surface);
        assert_eq!(surface_of(&bar.registry), Some(LayoutSurface::Popup));

        // Trigger a reload for that module name — the same path a widget
        // script or config hot-reload drives.
        let setup = ModuleSetup::new(&bar.config, &bar.deps);
        let name = ModuleName::new("calendar");
        bar.registry.reload_module(&name, setup).unwrap();

        // The recorded site facts must survive the reload...
        assert_eq!(surface_of(&bar.registry), Some(LayoutSurface::Popup));
        // ...and the ModuleContext `reload_module` actually builds for the
        // rebuilt actor must carry that surface, not reset to
        // `ModuleContext::new`'s `LayoutSurface::Bar` default.
        let (ctx, _sender) = bar.registry.build_site_context(id, &bar.deps);
        assert_eq!(ctx.surface(), LayoutSurface::Popup);
    });
}

/// A registry over `home` that loaded the root and the module `name`, with its configuration
/// and the actor dependencies a reload needs.
fn loaded_with(
    home: &crate::test_utils::TestHome,
    name: &str,
) -> (ModuleRegistry, Config, TestDeps) {
    let toml = format!("[root]\nname = \"bar\"\nleft = [\"{name}\"]");
    let (registry, config) = loaded(home.env(), &toml);
    let deps = test_deps(&config);
    (registry, config, deps)
}

/// A reload hands back a sender for each site of the module, and records the sheets and
/// signals of the new script again.
#[test]
fn a_reload_rebuilds_each_site_of_the_module_and_records_its_styles_and_signals() {
    crate::test_utils::block_on(async {
        let home = crate::test_utils::TestHome::new("registry");
        let (mut registry, config, deps) = loaded_with(&home, "clock");
        let clock = ModuleName::new("clock");
        // A second site of the same module, embedded lazily under the first one.
        let configured = registry.name_to_ids[&clock][0];
        let nested = LazySite::new(configured, ModuleKey::from_name(clock.clone()));
        registry
            .spawn_module(nested, ModuleSetup::new(&config, &deps))
            .unwrap();
        registry.style_to_modules.clear();
        registry.active_signals.clear();
        let mut sites = registry.name_to_ids[&clock].clone();
        sites.sort_unstable_by_key(|id| id.value());
        assert_eq!(sites.len(), 2, "the module must sit at two sites");

        let senders = registry
            .reload_module(&clock, ModuleSetup::new(&config, &deps))
            .unwrap();

        let mut reloaded: Vec<ModuleId> = senders.keys().copied().collect();
        reloaded.sort_unstable_by_key(|id| id.value());
        assert_eq!(reloaded, sites);
        let sheet = StyleSheetName::new("clock").unwrap();
        assert_eq!(registry.modules_using_style(&sheet), [clock]);
        assert!(registry.active_signals.contains(&SignalKind::Time));
    });
}

#[test]
fn a_reload_reports_a_script_that_is_gone() {
    let home = crate::test_utils::TestHome::new("registry");
    write_user_module(home.home().as_path(), "gone.lua", PLAIN_MODULE);
    let (mut registry, config, deps) = loaded_with(&home, "gone");
    std::fs::remove_file(
        home.home()
            .as_path()
            .join(".config/cranky/modules/gone.lua"),
    )
    .unwrap();

    let gone = ModuleName::new("gone");
    let result = registry.reload_module(&gone, ModuleSetup::new(&config, &deps));
    assert!(matches!(result, Err(RegistryLoadError::ModuleNotFound(name)) if name == gone));
}

#[test]
fn a_reload_reports_a_failing_init() {
    let home = crate::test_utils::TestHome::new("registry");
    write_user_module(home.home().as_path(), "flaky.lua", PLAIN_MODULE);
    let (mut registry, config, deps) = loaded_with(&home, "flaky");
    write_user_module(home.home().as_path(), "flaky.lua", FAILING_INIT);

    let flaky = ModuleName::new("flaky");
    match registry.reload_module(&flaky, ModuleSetup::new(&config, &deps)) {
        Err(RegistryLoadError::ModuleInit {
            module_name,
            source,
        }) => {
            assert_eq!(module_name, flaky);
            assert!(source.to_string().contains("boom"), "{source}");
        }
        other => panic!(
            "expected an init error, got {:?}",
            other.map(|senders| senders.len())
        ),
    }
}

/// The site's recorded configuration pins the engine a reload uses.
#[test]
fn a_reload_reports_an_unsupported_engine() {
    let home = crate::test_utils::TestHome::new("registry");
    let (mut registry, config, deps) = loaded_with(&home, "clock");
    let clock = ModuleName::new("clock");
    let id = registry.name_to_ids[&clock][0];
    let python = EngineSelection::Explicit(EngineId::new("python"));
    let pinned = ModuleConfig::new(clock.clone(), ModuleOptions::default()).with_engine(python);
    registry.module_configs.insert(id, pinned);

    let result = registry.reload_module(&clock, ModuleSetup::new(&config, &deps));
    let expected = RegistryLoadError::UnsupportedEngine {
        engine: "python".to_string(),
        module_name: clock,
    };
    assert_eq!(result.err(), Some(expected));
}

#[test]
fn a_reload_reports_a_home_that_is_no_longer_a_directory_as_internal() {
    let home = crate::test_utils::TestHome::new("registry");
    let (mut registry, config, deps) = loaded_with(&home, "clock");
    let home_dir = home.home().as_path().clone();
    std::fs::remove_dir_all(&home_dir).unwrap();
    std::fs::write(&home_dir, "").unwrap();

    let result =
        registry.reload_module(&ModuleName::new("clock"), ModuleSetup::new(&config, &deps));
    std::fs::remove_file(&home_dir).unwrap();
    assert!(
        matches!(&result, Err(RegistryLoadError::Internal(message)) if message.contains("os error")),
        "{:?}",
        result.map(|senders| senders.len())
    );
}
