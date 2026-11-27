//! Reloading a module: every site configured with its name gets a fresh actor, on the
//! surface it was spawned on.
use super::super::{ModuleRegistry, ModuleSiteInfo};
use super::LoadedBar;
use crate::features::module_runtime::ports::{ModuleRegistryPort, ModuleSetup};
use crate::shared::primitives::{LayoutSurface, ModuleName};

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
