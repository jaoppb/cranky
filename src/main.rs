#![deny(unsafe_code)]
#![warn(clippy::type_complexity, clippy::needless_lifetimes)]

use cranky::app::adapter_supervisor::AdapterSupervisor;
use cranky::app::commands::{ChannelSystemSender, SystemCommand};
use cranky::app::state::CrankyApp;
use cranky::features::layout_engine::domain::DisplayCommand;
use cranky::features::module_runtime::ports::LayoutEvent;
use cranky::features::styling::ports::StyleLoaderPort;
use cranky::features::systray::adapters::SniAdapter;
use cranky::features::systray::ports::SniPort;
use cranky::features::vdom::domain::UiCommand;
use cranky::shared::config::adapters::ConfigAdapter;
use cranky::shared::events::signals::{SignalHub, SignalKind};
use cranky::shared::rendering::adapters::font::CosmicFontValidatorAdapter;
use cranky::shared::wayland::adapters::wayland::WaylandAdapter;
use std::sync::Arc;
use tracing::{error, info, info_span};

use tokio::sync::mpsc;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use cranky::shared::env::domain::AppEnvironment;
use cranky::shared::env::ports::EnvironmentPort;

fn init_tracing(env: &AppEnvironment) -> tracing_appender::non_blocking::WorkerGuard {
    let file_appender = tracing_appender::rolling::daily(
        env.xdg_cache_home().as_path().join("cranky"),
        "cranky.log",
    );
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(env.rust_log().as_str()));

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

/// Establishes the (boot-fatal) `DBus` connection, builds the on-demand
/// adapter supervisor around it, and eagerly `ensure`s every signal already
/// active from the initial config load. Systray is handled separately: its
/// `start()` needs `&mut self` on the very instance `CrankyApp::run` later
/// drives via `SniPort::trigger_action`, so it isn't part of the supervisor
/// — see `SPEC.md`'s Phase 3 write-up.
async fn init_secondary_adapters(
    hub: &Arc<SignalHub>,
    app_env: &Arc<AppEnvironment>,
    metrics_config: &cranky::features::metrics::domain::MetricsConfig,
    active_signals: &std::collections::HashSet<SignalKind>,
) -> Result<
    (
        cranky::shared::dbus::subscription_manager::DbusSubscriptionManager,
        SniAdapter,
        AdapterSupervisor,
    ),
    Box<dyn std::error::Error>,
> {
    let conn_adapter = cranky::shared::dbus::adapters::connection::ZbusConnectionAdapter::new()
        .connect()
        .await
        .map_err(|e| format!("DBus connection required: {e}"))?;
    let conn: Arc<dyn cranky::shared::dbus::ports::DbusConnectionPort> = Arc::new(conn_adapter);

    let dbus_manager =
        cranky::shared::dbus::subscription_manager::DbusSubscriptionManager::new(conn.clone(), hub);

    let mut supervisor = AdapterSupervisor::new(
        hub.clone(),
        app_env.clone(),
        metrics_config.clone(),
        conn.clone(),
    );
    for kind in [SignalKind::Mpris, SignalKind::Metrics, SignalKind::Hyprland, SignalKind::Time] {
        if active_signals.contains(&kind) {
            supervisor.ensure(kind).await;
        }
    }

    let mut sni_adapter = SniAdapter::new(hub.clone());
    if active_signals.contains(&SignalKind::Systray) {
        supervisor.mark_started(SignalKind::Systray);
        if let Err(e) = sni_adapter.start().await {
            error!("Failed to start SNI Watcher: {e:?}");
        }
    }

    Ok((dbus_manager, sni_adapter, supervisor))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env_adapter = cranky::shared::env::adapters::os::OsEnvironmentAdapter;
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
    let surface_manager: cranky::shared::wayland::ports::DynSurfaceManager =
        std::sync::Arc::new(surface_manager);

    let registry = Box::new(cranky::app::registry::ModuleRegistry::new(app_env.clone()));

    let canvas_factory =
        cranky::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new();

    let display_sender = std::sync::Arc::new(display_tx);
    let layout_sender = std::sync::Arc::new(layout_tx);
    let ui_sender = std::sync::Arc::new(ui_tx);

    let services = cranky::app::state::StateServices::new(
        hub.clone(),
        surface_manager,
        canvas_factory,
        display_sender,
        layout_sender,
        ui_sender,
    );
    let channels = cranky::app::state::StateChannels::new(
        display_rx,
        layout_rx,
        ui_rx,
        system_rx,
    );
    let mut app = CrankyApp::new(
        initial_config.clone(),
        services,
        channels,
        registry,
    )?;

    let active_signals = app.active_signals();

    // 3. Initialize secondary adapters, and wire the supervisor in for
    // later on-demand starts triggered by a lazily-spawned module.
    let (zbus_adapter, sni_adapter, adapter_supervisor) =
        init_secondary_adapters(&hub, &app_env, initial_config.metrics(), active_signals).await?;
    app.set_adapter_supervisor(adapter_supervisor);

    let _config_watcher = config_adapter.watch(&hub)?;

    let style_loader =
        cranky::features::styling::adapters::fs_loader::FsStyleLoader::new(app_env.clone());
    if let Err(e) = style_loader.ensure_builtin_styles() {
        error!("Failed to deploy builtin styles: {e}");
    }

    let style_system_tx = system_tx.clone();
    let _style_watcher = match style_loader.watch_styles(Arc::new(move |sheet| {
        let _ = style_system_tx.try_send(SystemCommand::ReloadStyle(sheet));
    })) {
        Ok(w) => Some(w),
        Err(e) => {
            error!("Failed to watch style directories: {e}");
            None
        }
    };

    let _script_watcher = cranky::app::builtins::BuiltinModules::watch_scripts(
        Arc::new(ChannelSystemSender::new(system_tx)),
        &app_env,
    )?;

    // 5. Start the Core App Orchestrator
    info!("Cranky started successfully.");
    app.run(wayland_adapter, zbus_adapter, sni_adapter).await?;

    Ok(())
}
