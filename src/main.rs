#![deny(unsafe_code)]

use tracing::{info, info_span};
mod config;
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
    let config_adapter = ConfigAdapter::new();
    let hyprland_adapter = HyprlandAdapter::new();
    
    // 2. Load Initial Configuration
    let initial_config = config_adapter.load_initial()?;
    
    // 3. Initialize Reactive Signal Hub
    let (hub, dirty_rx) = SignalHub::new(initial_config.clone());
    let hub = Arc::new(hub);

    // 4. Initialize Core Orchestrator
    let (command_tx, _command_rx) = mpsc::channel::<AppCommand>(100);
    let mut app = CrankyApp::new(
        hub.clone(),
        dirty_rx,
        initial_config,
        command_tx.clone()
    );

    // 5. Initialize Wayland Adapter
    let mut wayland_adapter = WaylandAdapter::new(hub.clone())?;

    // 6. Spawn Background Adapters
    let hub_for_hypr = hub.clone();
    tokio::spawn(async move {
        hyprland_adapter.run(hub_for_hypr).await;
    }.instrument(info_span!("hyprland_adapter")));

    let hub_for_config = hub.clone();
    let _config_watcher = config_adapter.watch(hub_for_config)?;

    // 7. Start the Core App Orchestrator
    info!("Cranky started successfully.");
    app.run(&mut wayland_adapter).await?;

    Ok(())
}
