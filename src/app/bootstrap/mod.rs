//! Composition root: reads the environment, builds every adapter and the
//! application, and drives it until it exits.

mod channels;
mod error;
mod secondary;
mod tracing_setup;
mod watchers;

use crate::app::commands::SystemCommand;
use crate::app::state::CrankyApp;
use crate::features::layout_engine::domain::DisplayCommand;
use crate::features::module_runtime::ports::LayoutEvent;
use crate::features::styling::domain::StylingError;
use crate::features::vdom::domain::UiCommand;
use crate::shared::config::adapters::ConfigAdapter;
use crate::shared::env::domain::AppEnvironment;
use crate::shared::env::ports::EnvironmentPort;
use crate::shared::events::signals::SignalHub;
use crate::shared::rendering::adapters::font::CosmicFontValidatorAdapter;
use crate::shared::wayland::adapters::wayland::WaylandAdapter;
use channels::BootChannels;
pub use error::BootError;
use secondary::init_secondary_adapters;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, info_span};
use tracing_setup::init_tracing;
use watchers::start_file_watchers;

/// The application the bar runs: the module registry, the tiny-skia canvas factory and the
/// senders of the boot channels.
type BarApp = CrankyApp<
    crate::app::registry::ModuleRegistry,
    crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory,
    mpsc::Sender<LayoutEvent>,
    mpsc::Sender<DisplayCommand>,
    mpsc::Sender<UiCommand>,
    crate::app::registry::WatchLayoutSender,
>;

/// Runs the bar on a single-threaded Tokio runtime until the application
/// loop exits.
///
/// # Errors
///
/// Returns a [`BootError`] when a boot-fatal step fails: building the
/// runtime, reading the environment, loading or watching the configuration,
/// connecting to Wayland, building the application, watching the scripts, or
/// the application loop.
pub fn run() -> Result<(), BootError> {
    build_runtime()?.block_on(async {
        let env_adapter = crate::shared::env::adapters::system::OsEnvironmentAdapter;
        let app_env = Arc::new(env_adapter.read_environment()?);

        let _guard = init_tracing(&app_env);

        let main_span = info_span!("cranky_main");
        let _main_enter = main_span.enter();

        info!("Starting Cranky bar (Hexagonal + Reactive)...");

        // 1. Initial configuration and Core Hub
        let (config_adapter, hub) = load_config(&app_env)?;

        // 2. Initialize Wayland and Core App
        let (wayland_adapter, mut app, system_tx) = build_core(&hub, &app_env)?;

        // 3. Initialize secondary adapters, and wire the supervisor in for
        // later on-demand starts triggered by a lazily-spawned module. It
        // reads its `[metrics]` settings from the hub, which still holds the
        // initial configuration: the config watcher starts below.
        let (bus, adapter_supervisor) =
            init_secondary_adapters(&hub, &app_env, app.active_signals()).await?;
        app.set_adapter_supervisor(adapter_supervisor);

        let _config_watcher = config_adapter.watch(&hub)?;
        let _file_watchers = start_file_watchers(&app_env, system_tx)?;

        // 5. Start the Core App Orchestrator
        info!("Cranky started successfully.");
        app.run(wayland_adapter, bus).await?;

        Ok(())
    })
}

/// The single-threaded Tokio runtime, with every driver enabled, the bar runs on.
///
/// # Errors
///
/// Returns [`BootError::Runtime`] when the runtime cannot be built.
fn build_runtime() -> Result<tokio::runtime::Runtime, BootError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(BootError::Runtime)
}

/// The configuration adapter, with fonts checked by cosmic-text, and a hub holding the
/// initial configuration it loads.
///
/// # Errors
///
/// Returns a [`BootError`] when the initial configuration cannot be loaded.
fn load_config(
    app_env: &Arc<AppEnvironment>,
) -> Result<(ConfigAdapter<CosmicFontValidatorAdapter>, Arc<SignalHub>), BootError> {
    let font_validator = CosmicFontValidatorAdapter::new();
    let config_adapter = ConfigAdapter::new(font_validator, app_env);
    let initial_config = config_adapter.load_initial()?;

    let hub = Arc::new(SignalHub::new(initial_config));
    Ok((config_adapter, hub))
}

/// Opens the boot channels and builds over `hub` the Wayland adapter, then the module
/// registry and the application, which loads and spawns the modules; the system channel's
/// sender comes back for the file watchers.
///
/// # Errors
///
/// Returns a [`BootError`] when Wayland cannot be reached or the application cannot load its
/// modules.
fn build_core(
    hub: &Arc<SignalHub>,
    app_env: &Arc<AppEnvironment>,
) -> Result<(WaylandAdapter, BarApp, mpsc::Sender<SystemCommand>), BootError> {
    let channels = BootChannels::new();
    let (wayland_adapter, surface_manager) =
        WaylandAdapter::new(hub.clone(), channels.display_tx().clone(), app_env.clone())?;
    let registry = Box::new(crate::app::registry::ModuleRegistry::new(app_env.clone()));
    let (deps, state_channels, system_tx) = channels.into_app_parts(hub, Arc::new(surface_manager));
    // The app starts from the configuration the hub holds, the initial one: it must be built
    // before the config watcher starts.
    let app = CrankyApp::new(deps, state_channels, registry)?;
    Ok((wayland_adapter, app, system_tx))
}

/// Logs that the builtin style sheets could not be deployed; logged from here so the event
/// keeps the `cranky::app::bootstrap` target it always had.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_style_deploy_failed(error: &StylingError) {
    error!("Failed to deploy builtin styles: {error}");
}

/// Logs that the style directories could not be watched, under the `cranky::app::bootstrap`
/// target too.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_style_watch_failed(error: &StylingError) {
    error!("Failed to watch style directories: {error}");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes `toml` as the user configuration file of `home`.
    fn write_config(home: &crate::test_utils::TestHome, toml: &str) {
        let dir = home.home().as_path().join(".config/cranky");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("config.toml"), toml).unwrap();
    }

    #[test]
    fn test_the_hub_starts_from_the_user_configuration() {
        let home = crate::test_utils::TestHome::new("bootstrap");
        write_config(&home, "[metrics]\nupdate_interval_ms = 2500\n");

        let (_config_adapter, hub) = load_config(&home.env()).unwrap();

        let config = hub.config_rx().borrow().clone();
        assert_eq!(config.metrics().update_interval_ms().value(), 2500);
    }

    #[test]
    fn test_an_unreadable_configuration_stops_the_boot() {
        let home = crate::test_utils::TestHome::new("bootstrap");
        write_config(&home, "this is = = not toml");

        let error = load_config(&home.env()).err().unwrap();

        assert!(matches!(error, BootError::Config(_)), "{error:?}");
    }

    #[test]
    fn test_the_runtime_is_single_threaded_with_every_driver() {
        let runtime = build_runtime().unwrap();

        assert_eq!(
            runtime.handle().runtime_flavor(),
            tokio::runtime::RuntimeFlavor::CurrentThread
        );
        runtime.block_on(async {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        });
    }
}
