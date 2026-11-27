//! Loading a configuration: the root, its configured children and the modules its sections
//! name, each at its own site.
use super::super::{ModuleRegistry, ModuleSiteInfo};
use super::{TestRegistryPort, config_from, env_at, loaded};
use crate::features::module_runtime::ports::RegistryLoadError;
use crate::shared::config::domain::ModuleConfig;
use crate::shared::primitives::{LayoutSurface, ModuleId, ModuleKey, ModuleName, ModuleOptions};

#[test]
fn test_module_registry_load() {
    let home = crate::test_utils::TestHome::new("registry");
    let (registry, _config) = loaded(
        home.env(),
        r#"
            [root]
            name = "bar"
            left = ["clock"]
        "#,
    );
    assert_eq!(TestRegistryPort::module_ids(&registry).len(), 2);
    assert!(TestRegistryPort::root_module(&registry).is_some());
}

/// The root sits on the bar with no parent, and every child it loads, whether configured
/// under `[modules]` or only named in a root section, sits on the bar under the root.
#[test]
fn test_module_registry_load_places_children_under_the_root() {
    let home = crate::test_utils::TestHome::new("registry");
    let (registry, _config) = loaded(
        home.env(),
        r#"
            [root]
            name = "bar"
            left = ["clock"]

            [modules.calendar]
        "#,
    );

    let root = TestRegistryPort::root_module(&registry).unwrap();
    assert_eq!(TestRegistryPort::parent_of(&registry, root), None);
    for name in ["clock", "calendar"] {
        assert_child_of_root(&registry, root, &ModuleName::new(name));
    }
}

/// Checks the one module `name` sits on the bar under `root` and resolves from there.
fn assert_child_of_root(registry: &ModuleRegistry, root: ModuleId, name: &ModuleName) {
    let [child] = registry.name_to_ids[name].as_slice() else {
        panic!("expected one {name} module");
    };
    assert_eq!(TestRegistryPort::parent_of(registry, *child), Some(root));
    let key = ModuleKey::from_name(name.clone());
    assert_eq!(
        TestRegistryPort::resolve_site(registry, Some(root), &key),
        Some(*child)
    );
    assert_eq!(
        registry.sites.get(child).map(ModuleSiteInfo::surface),
        Some(LayoutSurface::Bar)
    );
}

#[test]
fn test_per_site_module_identity() {
    let mut registry = ModuleRegistry::new(env_at(std::path::Path::new("/tmp")));
    let config = config_from("[root]\nname = \"bar\"");
    let calendar_cfg = ModuleConfig::new(ModuleName::new("calendar"), ModuleOptions::default());
    let (parent_a, parent_b) = (ModuleId::new(100), ModuleId::new(200));

    let mut load_under = |parent: ModuleId| {
        let (id, _) = registry.load_single_module(&calendar_cfg, &config).unwrap();
        let site = ModuleSiteInfo::configured(calendar_cfg.name().clone(), Some(parent));
        registry.record_site(id, site);
        id
    };
    let (id_a, id_b) = (load_under(parent_a), load_under(parent_b));

    // Two sites embedding the same module name mint distinct ModuleIds.
    assert_ne!(id_a, id_b);

    // Each site resolves to its own actor, keyed by its own parent —
    // never to the other parent's child of the same name.
    let key = ModuleKey::from_name(ModuleName::new("calendar"));
    let resolve =
        |parent: u32| TestRegistryPort::resolve_site(&registry, Some(ModuleId::new(parent)), &key);
    assert_eq!(
        (resolve(100), resolve(200), resolve(999)),
        (Some(id_a), Some(id_b), None)
    );

    let parent_of = |id| registry.sites.get(&id).and_then(ModuleSiteInfo::parent);
    assert_eq!(
        (parent_of(id_a), parent_of(id_b)),
        (Some(parent_a), Some(parent_b))
    );
}

#[test]
fn test_module_registry_load_errors() {
    let home = crate::test_utils::TestHome::new("registry");
    let mut registry = ModuleRegistry::new(home.env());
    let config = config_from(
        r#"
            [root]
            name = "bar"
            left = ["nonexistent"]
        "#,
    );

    let result = TestRegistryPort::load(&mut registry, &config);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        RegistryLoadError::ModuleNotFound(_)
    ));
}
