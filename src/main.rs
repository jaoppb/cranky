#![deny(unsafe_code)]

use tracing::{info, info_span};
mod core;
mod domain;
mod modules;
mod ports;
mod utils;
mod adapters;

#[cfg(test)]
#[macro_use]
pub mod test_utils;

use crate::domain::signals::SignalHub;
use crate::domain::app::CrankyApp;
use crate::adapters::wayland::WaylandAdapter;
use crate::adapters::hyprland::HyprlandAdapter;
use crate::adapters::config::ConfigAdapter;
use crate::adapters::font::CosmicFontValidatorAdapter;
use crate::domain::commands::AppCommand;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::Instrument;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let main_span = info_span!("cranky_main");
    let _main_enter = main_span.enter();

    info!("Starting Cranky bar (Hexagonal + Reactive)...");

    // 1. Initialize Infrastructure Adapters
    let font_validator = CosmicFontValidatorAdapter::new();
    let config_adapter = ConfigAdapter::new(font_validator);
    let hyprland_adapter = HyprlandAdapter::new();
    
    // 2. Load Initial Configuration
    let initial_config = config_adapter.load_initial()?;
    
    // 3. Initialize Reactive Signal Hub
    let (hub, dirty_rx) = SignalHub::new(initial_config.clone());
    let hub = Arc::new(hub);

    // 4. Initialize Core Orchestrator
    let (command_tx, command_rx) = mpsc::channel::<AppCommand>(100);
    let mut app = CrankyApp::new(
        hub.clone(),
        dirty_rx,
        initial_config,
        command_rx
    );

    // 4. Initialize display server port
    let mut wayland_adapter = WaylandAdapter::new(hub.clone(), command_tx.clone())?;

    // 6. Spawn Background Adapters
    let hub_for_hypr = hub.clone();
    tokio::spawn(async move {
        hyprland_adapter.run(hub_for_hypr).await;
    }.instrument(info_span!("hyprland_adapter")));

    let hub_for_time = hub.clone();
    tokio::spawn(async move {
        loop {
            let now = chrono::Local::now();
            let ms_until_next_sec = 1000 - now.timestamp_subsec_millis() as u64;
            tokio::time::sleep(std::time::Duration::from_millis(ms_until_next_sec)).await;
            let _ = hub_for_time.time_tx().send(chrono::Local::now());
        }
    }.instrument(info_span!("time_adapter")));

    let hub_for_config = hub.clone();
    let _config_watcher = config_adapter.watch(hub_for_config)?;

    // 7. Start the Core App Orchestrator
    info!("Cranky started successfully.");
    app.run(&mut wayland_adapter).await?;

    Ok(())
}
