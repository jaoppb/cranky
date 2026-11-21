//! Composition root: reads the environment, builds every adapter and the
//! application, and drives it until it exits.

mod error;
mod secondary;
mod tracing_setup;

use crate::app::commands::{ChannelSystemSender, SystemCommand};
use crate::app::state::CrankyApp;
use crate::features::layout_engine::domain::DisplayCommand;
use crate::features::module_runtime::ports::LayoutEvent;
use crate::features::styling::ports::StyleLoaderPort;
use crate::features::vdom::domain::UiCommand;
use crate::shared::config::adapters::ConfigAdapter;
use crate::shared::env::ports::EnvironmentPort;
use crate::shared::events::signals::SignalHub;
use crate::shared::rendering::adapters::font::CosmicFontValidatorAdapter;
use crate::shared::wayland::adapters::wayland::WaylandAdapter;
pub use error::BootError;
use secondary::init_secondary_adapters;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, info_span};
use tracing_setup::init_tracing;

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
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(BootError::Runtime)?;
    runtime.block_on(async {
        let env_adapter = crate::shared::env::adapters::system::OsEnvironmentAdapter;
        let app_env = std::sync::Arc::new(env_adapter.read_environment()?);

        let _guard = init_tracing(&app_env);

        let main_span = info_span!("cranky_main");
        let _main_enter = main_span.enter();

        info!("Starting Cranky bar (Hexagonal + Reactive)...");

        // 1. Initial configuration and Core Hub
        let font_validator = CosmicFontValidatorAdapter::new();
        let config_adapter = ConfigAdapter::new(font_validator, &app_env);
        let initial_config = config_adapter.load_initial()?;

        let hub = Arc::new(SignalHub::new(initial_config.clone()));

        // 2. Initialize Wayland and Core App
        let (display_tx, display_rx) = mpsc::channel::<DisplayCommand>(100);
        let (layout_tx, layout_rx) = mpsc::channel::<LayoutEvent>(100);
        let (ui_tx, ui_rx) = mpsc::channel::<UiCommand>(100);
        let (system_tx, system_rx) = mpsc::channel::<SystemCommand>(100);

        let (wayland_adapter, surface_manager) =
            WaylandAdapter::new(hub.clone(), display_tx.clone(), app_env.clone())?;
        let surface_manager: crate::shared::wayland::ports::DynSurfaceManager =
            std::sync::Arc::new(surface_manager);

        let registry = Box::new(crate::app::registry::ModuleRegistry::new(app_env.clone()));

        let canvas_factory =
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new();

        let display_sender = std::sync::Arc::new(display_tx);
        let layout_sender = std::sync::Arc::new(layout_tx);
        let ui_sender = std::sync::Arc::new(ui_tx);

        let services = crate::app::state::StateServices::new(
            hub.clone(),
            surface_manager,
            canvas_factory,
            display_sender,
            layout_sender,
            ui_sender,
        );
        let channels =
            crate::app::state::StateChannels::new(display_rx, layout_rx, ui_rx, system_rx);
        let mut app = CrankyApp::new(initial_config.clone(), services, channels, registry)?;

        let active_signals = app.active_signals();

        // 3. Initialize secondary adapters, and wire the supervisor in for
        // later on-demand starts triggered by a lazily-spawned module.
        let (zbus_adapter, sni_adapter, adapter_supervisor) =
            init_secondary_adapters(&hub, &app_env, initial_config.metrics(), active_signals)
                .await?;
        app.set_adapter_supervisor(adapter_supervisor);

        let _config_watcher = config_adapter.watch(&hub)?;

        let style_loader =
            crate::features::styling::adapters::fs_loader::FsStyleLoader::new(app_env.clone());
        if let Err(error) = style_loader.ensure_builtin_styles() {
            error!("Failed to deploy builtin styles: {error}");
        }

        let style_system_tx = system_tx.clone();
        let _style_watcher = match style_loader.watch_styles(Arc::new(move |sheet| {
            let _ = style_system_tx.try_send(SystemCommand::ReloadStyle(sheet));
        })) {
            Ok(watcher) => Some(watcher),
            Err(error) => {
                error!("Failed to watch style directories: {error}");
                None
            }
        };

        let _script_watcher = crate::app::builtins::BuiltinModules::watch_scripts(
            Arc::new(ChannelSystemSender::new(system_tx)),
            &app_env,
        )?;

        // 5. Start the Core App Orchestrator
        info!("Cranky started successfully.");
        app.run(wayland_adapter, zbus_adapter, sni_adapter).await?;

        Ok(())
    })
}
