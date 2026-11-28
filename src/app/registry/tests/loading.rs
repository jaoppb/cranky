//! Loading a configuration: the root, its configured children and the modules its sections
//! name, each at its own site.
use super::super::{ModuleRegistry, ModuleSiteInfo};
use super::{FAILING_INIT, TestRegistryPort, config_from, env_at, loaded, write_user_module};
use crate::features::module_runtime::ports::RegistryLoadError;
use crate::features::styling::domain::StyleSheetName;
use crate::shared::config::domain::ModuleConfig;
use crate::shared::events::signals::SignalKind;
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
    let calendar = ModuleConfig::new(ModuleName::new("calendar"), ModuleOptions::default());
    let (parent_a, parent_b) = (ModuleId::new(100), ModuleId::new(200));

    let mut load_under = |parent: ModuleId| {
        let id = registry.load_single_module(&calendar, &config).unwrap().id;
        let site = ModuleSiteInfo::configured(calendar.name().clone(), Some(parent));
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

/// A disabled `[modules.*]` entry is not loaded.
#[test]
fn a_disabled_module_is_not_loaded() {
    let home = crate::test_utils::TestHome::new("registry");
    let toml = "[root]\nname = \"bar\"\n[modules.calendar]\nenable = false";
    let (registry, _config) = loaded(home.env(), toml);
    assert!(
        !registry
            .name_to_ids
            .contains_key(&ModuleName::new("calendar"))
    );
    assert_eq!(TestRegistryPort::module_ids(&registry).len(), 1);
}

/// A `[modules.<root>]` entry configures the root, which is loaded once, with its options.
#[test]
fn the_root_configured_under_modules_is_loaded_once_with_those_options() {
    let home = crate::test_utils::TestHome::new("registry");
    let toml = "[root]\nname = \"bar\"\n[modules.bar]\nlabel = \"from-modules\"";
    let (registry, config) = loaded(home.env(), toml);
    let bar = ModuleName::new("bar");
    let [root] = registry.name_to_ids[&bar].as_slice() else {
        panic!(
            "expected one bar module, got {:?}",
            registry.name_to_ids[&bar]
        );
    };
    assert_eq!(TestRegistryPort::root_module(&registry), Some(*root));
    let root_options = registry.module_configs.get(root).map(ModuleConfig::options);
    assert_eq!(
        root_options,
        config.modules().get(&bar).map(ModuleConfig::options)
    );
}

/// A root without a `[modules.<root>]` entry is configured from the `[root]` options.
#[test]
fn the_root_without_a_modules_entry_takes_the_root_options() {
    let home = crate::test_utils::TestHome::new("registry");
    let (registry, config) = loaded(home.env(), "[root]\nname = \"bar\"\nlabel = \"x\"");
    let root = TestRegistryPort::root_module(&registry).unwrap();
    let root_options = registry
        .module_configs
        .get(&root)
        .map(ModuleConfig::options);
    assert_eq!(root_options, Some(config.root().options()));
}

/// A name the sections repeat, or that `[modules]` already configures, is loaded once,
/// and a section item that is not a name is ignored.
#[test]
fn a_section_module_already_loaded_is_not_loaded_again() {
    let home = crate::test_utils::TestHome::new("registry");
    let toml = r#"
        [root]
        name = "bar"
        left = ["clock", 7, "calendar"]
        right = ["clock", "calendar"]
        [modules.clock]
    "#;
    let (registry, _config) = loaded(home.env(), toml);
    for name in ["clock", "calendar"] {
        assert_eq!(
            registry.name_to_ids[&ModuleName::new(name)].len(),
            1,
            "{name}"
        );
    }
    assert_eq!(TestRegistryPort::module_ids(&registry).len(), 3);
}

#[test]
fn an_unsupported_engine_is_reported_with_the_engine_and_module() {
    let home = crate::test_utils::TestHome::new("registry");
    let mut registry = ModuleRegistry::new(home.env());
    let toml = "[root]\nname = \"bar\"\n[modules.clock]\nengine = \"python\"";
    let result = TestRegistryPort::load(&mut registry, &config_from(toml));
    assert_eq!(
        result,
        Err(RegistryLoadError::UnsupportedEngine {
            engine: "python".to_string(),
            module_name: ModuleName::new("clock"),
        })
    );
}

/// A home the built-in scripts cannot be deployed under is an internal error.
#[test]
fn a_home_that_is_not_a_directory_is_an_internal_error() {
    let home = crate::test_utils::TestHome::new("registry");
    let not_a_dir = home.home().as_path().join("not-a-directory");
    std::fs::write(&not_a_dir, "").unwrap();
    let mut registry = ModuleRegistry::new(env_at(&not_a_dir));
    let result = TestRegistryPort::load(&mut registry, &config_from("[root]\nname = \"bar\""));
    assert!(
        matches!(&result, Err(RegistryLoadError::Internal(message)) if message.contains("os error")),
        "{result:?}"
    );
}

#[test]
fn a_failing_init_is_reported_with_the_module_name_and_its_error() {
    let home = crate::test_utils::TestHome::new("registry");
    write_user_module(home.home().as_path(), "failing.lua", FAILING_INIT);
    let mut registry = ModuleRegistry::new(home.env());
    let toml = "[root]\nname = \"bar\"\nleft = [\"failing\"]";
    match TestRegistryPort::load(&mut registry, &config_from(toml)) {
        Err(RegistryLoadError::ModuleInit {
            module_name,
            source,
        }) => {
            assert_eq!(module_name, ModuleName::new("failing"));
            assert!(source.to_string().contains("boom"), "{source}");
        }
        other => panic!("expected an init error, got {other:?}"),
    }
}

/// A Lua module script subscribing to D-Bus and declaring the `watcher` sheet.
const WATCHER: &str = r#"
    function metadata()
        return { subscriptions = { { type = "dbus" } }, styles = { "watcher" } }
    end
    function render(monitor) return ui.text("") end
"#;

/// Loading a module records its D-Bus subscriptions, its signals, the sheets it uses and
/// its configuration under its name.
#[test]
fn a_loaded_module_records_its_subscriptions_styles_and_configuration() {
    let home = crate::test_utils::TestHome::new("registry");
    write_user_module(home.home().as_path(), "watcher.lua", WATCHER);
    let (registry, _config) = loaded(home.env(), "[root]\nname = \"bar\"\nleft = [\"watcher\"]");
    let watcher = ModuleName::new("watcher");
    let [id] = registry.name_to_ids[&watcher].as_slice() else {
        panic!("expected one watcher module");
    };
    assert_eq!(registry.dbus_subscriptions.len(), 1);
    let signals = TestRegistryPort::active_signal_subscriptions(&registry);
    assert!(signals.contains(&SignalKind::DBus), "{signals:?}");
    let sheet = StyleSheetName::new("watcher").unwrap();
    assert_eq!(
        registry.modules_using_style(&sheet),
        std::slice::from_ref(&watcher)
    );
    let configured = registry.module_configs.get(id).map(ModuleConfig::name);
    assert_eq!(configured, Some(&watcher));
}

/// An id is minted before the module is looked up, so a failed load still uses one up.
#[test]
fn a_failed_load_still_uses_up_an_id() {
    let mut registry = ModuleRegistry::new(env_at(std::path::Path::new("/tmp")));
    let config = config_from("[root]\nname = \"bar\"");
    let module = |name: &str| ModuleConfig::new(ModuleName::new(name), ModuleOptions::default());
    assert!(
        registry
            .load_single_module(&module("does-not-exist"), &config)
            .is_err()
    );
    let id = registry
        .load_single_module(&module("calendar"), &config)
        .unwrap()
        .id;
    assert_eq!(id, ModuleId::new(1));
}

#[test]
fn loading_a_module_logs_the_sheets_it_depends_on() {
    let home = crate::test_utils::TestHome::new("registry");
    let mut registry = ModuleRegistry::new(home.env());
    let config = config_from("[root]\nname = \"bar\"\nleft = [\"clock\"]");
    let logs = crate::features::module_runtime::test_support::capture_logs(|| {
        TestRegistryPort::load(&mut registry, &config).unwrap();
    });
    let clock = registry.name_to_ids[&ModuleName::new("clock")][0];
    let expected = format!(
        "DEBUG cranky::app::registry: Registered module style dependencies module=clock \
         id={clock} styles=[\"clock\"]"
    );
    assert!(logs.contains(&expected), "{logs}");
}
