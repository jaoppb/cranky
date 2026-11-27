//! Tests for `ModuleRegistry`: loading a configuration, spawning and lazily spawning actors,
//! reloading a module, and the style resolver each actor gets.
mod lifecycle;
mod loading;
mod reload;

use super::{ModuleRegistry, WatchLayoutSender};
use crate::features::module_runtime::ports::{
    LazySite, ModuleHandles, ModuleRegistryPort, ModuleRuntimeDependencies, ModuleSenders,
    ModuleSetup, RegistryLoadError,
};
use crate::features::module_runtime::test_support::{
    MockDisplaySender, MockLayoutSender, MockSurfaceManager, MockUiSender,
};
use crate::shared::config::adapters::dto::ConfigDto;
use crate::shared::config::domain::Config;
use crate::shared::env::domain::AppEnvironment;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::{ModuleId, ModuleKey, ModuleName};
use crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory;
use crate::shared::rendering::ports::font::FontValidatorPort;
use std::sync::Arc;

/// Accepts every font family, so a configuration parses without a font lookup.
struct MockValidator;

impl FontValidatorPort for MockValidator {
    fn is_valid_family(&self, _family: &str) -> bool {
        true
    }
}

/// The registry port as `CrankyApp` drives it, with the mock senders the tests use.
pub(super) type TestRegistryPort = dyn ModuleRegistryPort<
        TinySkiaCanvasFactory,
        MockLayoutSender,
        MockDisplaySender,
        MockUiSender,
        WatchLayoutSender,
    >;

/// The dependencies an actor spawned by a test is built with.
pub(super) type TestDeps = ModuleRuntimeDependencies<
    TinySkiaCanvasFactory,
    MockLayoutSender,
    MockDisplaySender,
    MockUiSender,
>;

/// The configuration `toml` describes.
pub(super) fn config_from(toml: &str) -> Config {
    let dto: ConfigDto = toml::from_str(toml).unwrap();
    dto.into_domain(&MockValidator)
}

/// An environment whose home, cache and runtime directories are all `home`.
pub(super) fn env_at(home: &std::path::Path) -> Arc<AppEnvironment> {
    Arc::new(AppEnvironment::new(
        crate::shared::env::domain::BaseDirectories::new(
            crate::shared::env::domain::HomeDir::new(home.to_path_buf()),
            crate::shared::env::domain::XdgCacheHome::new(home.to_path_buf()),
            crate::shared::env::domain::XdgRuntimeDir::new(home.to_path_buf()),
        ),
        crate::shared::env::domain::RustLog::new(String::new()),
        None,
    ))
}

/// A fresh registry over `env` that has loaded `toml`, with the configuration it loaded.
pub(super) fn loaded(env: Arc<AppEnvironment>, toml: &str) -> (ModuleRegistry, Config) {
    let mut registry = ModuleRegistry::new(env);
    let config = config_from(toml);
    TestRegistryPort::load(&mut registry, &config).unwrap();
    (registry, config)
}

/// Actor dependencies over a hub for `config`, a surface manager that does nothing and
/// senders that drop what they get.
pub(super) fn test_deps(config: &Config) -> TestDeps {
    ModuleRuntimeDependencies::new(
        ModuleHandles::new(
            Arc::new(SignalHub::new(config.clone())),
            Arc::new(MockSurfaceManager),
            ModuleSenders::new(
                Arc::new(MockLayoutSender),
                Arc::new(MockDisplaySender),
                Arc::new(MockUiSender),
            ),
        ),
        TinySkiaCanvasFactory::new(),
    )
}

/// A configuration with a `bar` root and nothing else.
pub(super) const BAR_ONLY: &str = "[root]\nname = \"bar\"";

/// A registry over a `/tmp` home that loaded a bare `bar` root, with the configuration it
/// loaded, the root's id and the dependencies its actors are spawned with.
pub(super) struct LoadedBar {
    pub(super) registry: ModuleRegistry,
    pub(super) config: Config,
    pub(super) root: ModuleId,
    pub(super) deps: TestDeps,
}

impl LoadedBar {
    pub(super) fn new() -> Self {
        let (registry, config) = loaded(env_at(std::path::Path::new("/tmp")), BAR_ONLY);
        let root = TestRegistryPort::root_module(&registry).unwrap();
        let deps = test_deps(&config);
        Self {
            registry,
            config,
            root,
            deps,
        }
    }

    /// The site of the module `name` embedded under the root, by name alone.
    pub(super) fn site_named(&self, name: &str) -> LazySite {
        LazySite::new(self.root, ModuleKey::from_name(ModuleName::new(name)))
    }

    /// Spawns `site` lazily and returns the id it was given.
    pub(super) fn spawn(&mut self, site: LazySite) -> Result<ModuleId, RegistryLoadError> {
        self.registry
            .spawn_module(site, ModuleSetup::new(&self.config, &self.deps))
            .map(|spawned| spawned.id())
    }
}
