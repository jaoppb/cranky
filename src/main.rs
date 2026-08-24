#![deny(unsafe_code)]
#![warn(clippy::type_complexity, clippy::needless_lifetimes)]

use tracing::{error, info, info_span};
pub mod app;
pub mod features;
pub mod shared;
mod utils;

#[cfg(test)]
#[macro_use]
pub mod test_utils;

use crate::app::commands::AppCommand;
use crate::app::state::CrankyApp;
use crate::features::metrics::adapters::SysinfoAdapter;
use crate::features::systray::adapters::SniAdapter;
use crate::features::systray::ports::SniPort;
use crate::features::workspaces::adapters::hyprland::HyprlandAdapter;
use crate::shared::config::adapters::ConfigAdapter;
use crate::shared::events::signals::SignalHub;
use crate::shared::rendering::adapters::font::CosmicFontValidatorAdapter;
use crate::shared::wayland::adapters::wayland::WaylandAdapter;
use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::Instrument;

use crate::shared::env::domain::AppEnvironment;
use crate::shared::env::ports::EnvironmentPort;

struct MainCommandSender(mpsc::Sender<AppCommand>);
impl crate::features::module_runtime::ports::CommandSender for MainCommandSender {
    fn send_command(&self, cmd: AppCommand) {
        let _ = self.0.try_send(cmd);
    }
}

fn init_tracing(env: &AppEnvironment) -> tracing_appender::non_blocking::WorkerGuard {
    let file_appender = tracing_appender::rolling::daily(
        env.xdg_cache_home().as_path().to_string_lossy().to_string() + "/cranky",
        "cranky.log",
    );
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(env.rust_log().as_str()));

    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stdout))
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false),
        )
        .init();

    guard
}

async fn init_secondary_adapters(
    hub: &Arc<SignalHub>,
    metrics_config: &crate::features::metrics::domain::MetricsConfig,
) -> Result<
    (
        crate::shared::dbus::subscription_manager::DbusSubscriptionManager,
        SniAdapter,
    ),
    Box<dyn std::error::Error>,
> {
    let conn_adapter = crate::shared::dbus::adapters::connection::ZbusConnectionAdapter::new()
        .connect()
        .await
        .map_err(|e| format!("DBus connection required: {}", e))?;
    let conn: Arc<dyn crate::shared::dbus::ports::DbusConnectionPort> = Arc::new(conn_adapter);

    let dbus_manager =
        crate::shared::dbus::subscription_manager::DbusSubscriptionManager::new(conn.clone(), hub);

    let mut mpris_adapter =
        crate::features::mpris::adapters::zbus::ZbusMprisAdapter::new(conn.clone(), hub);
    if let Err(e) = mpris_adapter.start_watching().await {
        error!("Failed to start MPRIS watcher: {}", e);
    }

    let mut sni_adapter = SniAdapter::new(hub.clone());
    if let Err(e) = sni_adapter.start().await {
        error!("Failed to start SNI Watcher: {:?}", e);
    }

    let metrics_adapter = SysinfoAdapter::new(metrics_config.clone(), hub.clone());
    metrics_adapter.start().await;

    Ok((dbus_manager, sni_adapter))
}

fn spawn_background_tasks(hub: Arc<SignalHub>, hyprland_adapter: HyprlandAdapter) {
    let hub_for_hypr = hub.clone();
    tokio::spawn(
        async move {
            hyprland_adapter.run(hub_for_hypr).await;
        }
        .instrument(info_span!("hyprland_adapter")),
    );

    let hub_for_time = hub.clone();
    tokio::spawn(
        async move {
            loop {
                let now = chrono::Local::now();
                let ms_until_next_sec = 1000 - now.timestamp_subsec_millis() as u64;
                tokio::time::sleep(std::time::Duration::from_millis(ms_until_next_sec)).await;
                let _ = hub_for_time.time_tx().send(chrono::Local::now());
            }
        }
        .instrument(info_span!("time_adapter")),
    );
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env_adapter = crate::shared::env::adapters::os::OsEnvironmentAdapter;
    let app_env = std::sync::Arc::new(env_adapter.read_environment()?);

    let _guard = init_tracing(&app_env);

    let main_span = info_span!("cranky_main");
    let _main_enter = main_span.enter();

    info!("Starting Cranky bar (Hexagonal + Reactive)...");

    // 1. Initial configuration and Core Hub
    let font_validator = CosmicFontValidatorAdapter::new();
    let config_adapter = ConfigAdapter::new(font_validator, app_env.clone());
    let initial_config = config_adapter.load_initial()?;

    let hub = Arc::new(SignalHub::new(initial_config.clone()));

    // 2. Initialize Wayland and Core App
    let (command_tx, command_rx) = mpsc::channel::<AppCommand>(100);

    let (wayland_adapter, surface_manager) =
        WaylandAdapter::new(hub.clone(), command_tx.clone(), app_env.clone())?;
    let surface_manager: crate::shared::wayland::ports::DynSurfaceManager =
        std::sync::Arc::new(surface_manager);

    let registry = Box::new(crate::app::registry::ModuleRegistry::new(app_env.clone()));

    let canvas_factory = Arc::new(std::sync::Mutex::new(
        crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new(),
    ));

    let mut app = CrankyApp::new(
        hub.clone(),
        initial_config.clone(),
        command_rx,
        command_tx.clone(),
        surface_manager,
        canvas_factory,
        registry,
    )?;

    // 3. Initialize secondary adapters
    let (zbus_adapter, sni_adapter) =
        init_secondary_adapters(&hub, initial_config.metrics()).await?;

    // 4. Spawn background worker tasks
    let hyprland_adapter = HyprlandAdapter::new(app_env.clone());
    spawn_background_tasks(hub.clone(), hyprland_adapter);

    let hub_for_config = hub.clone();
    let _config_watcher = config_adapter.watch(hub_for_config)?;

    let style_loader =
        crate::features::styling::adapters::fs_loader::FsStyleLoader::new(app_env.clone());
    use crate::features::styling::ports::StyleLoaderPort;
    if let Err(e) = style_loader.ensure_builtin_styles() {
        error!("Failed to deploy builtin styles: {}", e);
    }

    let _style_watcher =
        match style_loader.watch_styles(Arc::new(MainCommandSender(command_tx.clone()))) {
            Ok(w) => Some(w),
            Err(e) => {
                error!("Failed to watch style directories: {}", e);
                None
            }
        };

    let _script_watcher = crate::app::builtins::BuiltinModules::watch_scripts(
        Arc::new(MainCommandSender(command_tx.clone())),
        &app_env,
    )?;

    // 5. Start the Core App Orchestrator
    info!("Cranky started successfully.");
    app.run(wayland_adapter, zbus_adapter, sni_adapter).await?;

    Ok(())
}
