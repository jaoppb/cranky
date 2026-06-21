#![deny(unsafe_code)]
#![warn(clippy::type_complexity, clippy::needless_lifetimes)]

use tracing::{error, info, info_span};
mod adapters;
mod core;
mod domain;
mod modules;
mod ports;
mod utils;

#[cfg(test)]
#[macro_use]
pub mod test_utils;

use crate::adapters::config::ConfigAdapter;
use crate::adapters::font::CosmicFontValidatorAdapter;
use crate::adapters::hyprland::HyprlandAdapter;
use crate::adapters::metrics::SysinfoAdapter;
use crate::adapters::sni::SniAdapter;
use crate::adapters::wayland::WaylandAdapter;
use crate::adapters::zbus::ZbusAdapter;
use crate::domain::app::CrankyApp;
use crate::domain::commands::AppCommand;
use crate::domain::signals::SignalHub;
use crate::ports::DBusPort;
use crate::ports::sni::SniPort;
use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::Instrument;

fn init_tracing() -> tracing_appender::non_blocking::WorkerGuard {
    let file_appender = tracing_appender::rolling::daily(
        std::env::var("XDG_CACHE_HOME").unwrap_or_else(|_| {
            let mut path = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            path.push_str("/.cache");
            path
        }) + "/cranky",
        "cranky.log",
    );
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = tracing_subscriber::EnvFilter::from_default_env().add_directive(
        std::env::var("RUST_LOG")
            .unwrap_or_else(|_| "cranky=info".to_string())
            .parse()
            .unwrap(),
    );

    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stdout)
                .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE),
        )
        .init();

    guard
}

async fn init_secondary_adapters(
    hub: &Arc<SignalHub>,
    metrics_config: &crate::domain::metrics::MetricsConfig,
) -> (ZbusAdapter, SniAdapter) {
    let mut zbus_adapter = ZbusAdapter::new(hub);
    if let Err(e) = zbus_adapter.connect().await {
        error!("Failed to connect to DBus: {}", e);
    }

    let mut sni_adapter = SniAdapter::new(hub.clone());
    if let Err(e) = sni_adapter.start().await {
        error!("Failed to start SNI Watcher: {:?}", e);
    }

    let metrics_adapter = SysinfoAdapter::new(metrics_config.clone(), hub.clone());
    metrics_adapter.start().await;

    (zbus_adapter, sni_adapter)
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
    let _guard = init_tracing();

    let main_span = info_span!("cranky_main");
    let _main_enter = main_span.enter();

    info!("Starting Cranky bar (Hexagonal + Reactive)...");

    // 1. Initial configuration and Core Hub
    let font_validator = CosmicFontValidatorAdapter::new();
    let config_adapter = ConfigAdapter::new(font_validator);
    let initial_config = config_adapter.load_initial()?;

    let hub = Arc::new(SignalHub::new(initial_config.clone()));

    // 2. Initialize Wayland and Core App
    let (command_tx, command_rx) = mpsc::channel::<AppCommand>(100);

    let (wayland_adapter, surface_manager) = WaylandAdapter::new(hub.clone(), command_tx.clone())?;
    let surface_manager: crate::ports::surface::DynSurfaceManager =
        std::sync::Arc::new(surface_manager);

    let registry = Box::new(crate::modules::ModuleRegistry::new());
    let mut app = CrankyApp::new(
        hub.clone(),
        initial_config.clone(),
        command_rx,
        command_tx.clone(),
        surface_manager,
        registry,
    )?;

    // 3. Initialize secondary adapters
    let (zbus_adapter, sni_adapter) = init_secondary_adapters(&hub, initial_config.metrics()).await;

    // 4. Spawn background worker tasks
    let hyprland_adapter = HyprlandAdapter::new();
    spawn_background_tasks(hub.clone(), hyprland_adapter);

    let hub_for_config = hub.clone();
    let _config_watcher = config_adapter.watch(hub_for_config)?;

    // 5. Start the Core App Orchestrator
    info!("Cranky started successfully.");
    app.run(wayland_adapter, zbus_adapter, sni_adapter).await?;

    Ok(())
}
