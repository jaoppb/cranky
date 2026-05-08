use crate::config::Config;
use crate::core::hyprland::{Workspace, Monitor};
use tokio::sync::{watch, broadcast, mpsc};

#[derive(Clone, Debug, PartialEq)]
pub struct HyprlandState {
    workspaces: Vec<Workspace>,
    monitors: Vec<Monitor>,
}

impl HyprlandState {
    pub fn new(workspaces: Vec<Workspace>, monitors: Vec<Monitor>) -> Self {
        Self { workspaces, monitors }
    }

    pub fn workspaces(&self) -> &[Workspace] {
        &self.workspaces
    }

    pub fn monitors(&self) -> &[Monitor] {
        &self.monitors
    }
}

#[derive(Clone, Debug)]
pub enum PointerEvent {
    Enter { target_id: u32, x: f64, y: f64 },
    Leave { target_id: u32 },
    Motion { target_id: u32, x: f64, y: f64 },
    Click { target_id: u32, x: f64, y: f64, button: u32 },
    Scroll { target_id: u32, axis: u32, value: f64 },
}

pub struct SignalHub {
    config: (watch::Sender<Config>, watch::Receiver<Config>),
    hyprland: (watch::Sender<HyprlandState>, watch::Receiver<HyprlandState>),
    time: (watch::Sender<chrono::DateTime<chrono::Local>>, watch::Receiver<chrono::DateTime<chrono::Local>>),
    pointer_events: broadcast::Sender<PointerEvent>,
    dirty_events: (mpsc::Sender<u32>, mpsc::Receiver<u32>),
}

impl SignalHub {
    pub fn new(initial_config: Config) -> Self {
        let config = watch::channel(initial_config);
        let hyprland = watch::channel(HyprlandState::new(Vec::new(), Vec::new()));
        let time = watch::channel(chrono::Local::now());
        let (pointer_events, _) = broadcast::channel(100);
        let dirty_events = mpsc::channel(100);

        Self {
            config,
            hyprland,
            time,
            pointer_events,
            dirty_events,
        }
    }

    pub fn config_tx(&self) -> watch::Sender<Config> {
        self.config.0.clone()
    }
    pub fn config_rx(&self) -> watch::Receiver<Config> {
        self.config.1.clone()
    }

    pub fn hyprland_tx(&self) -> watch::Sender<HyprlandState> {
        self.hyprland.0.clone()
    }
    pub fn hyprland_rx(&self) -> watch::Receiver<HyprlandState> {
        self.hyprland.1.clone()
    }

    pub fn time_tx(&self) -> watch::Sender<chrono::DateTime<chrono::Local>> {
        self.time.0.clone()
    }
    pub fn time_rx(&self) -> watch::Receiver<chrono::DateTime<chrono::Local>> {
        self.time.1.clone()
    }

    pub fn pointer_tx(&self) -> broadcast::Sender<PointerEvent> {
        self.pointer_events.clone()
    }
    pub fn subscribe_pointer(&self) -> broadcast::Receiver<PointerEvent> {
        self.pointer_events.subscribe()
    }

    pub fn dirty_tx(&self) -> mpsc::Sender<u32> {
        self.dirty_events.0.clone()
    }
    pub fn dirty_rx(&mut self) -> &mut mpsc::Receiver<u32> {
        &mut self.dirty_events.1
    }
}
