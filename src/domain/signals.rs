use crate::domain::config::Config;
use crate::core::hyprland::{Workspace, Monitor};
use crate::domain::{ModuleId, geometry::Point64};
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
    Enter { target_id: ModuleId, pos: Point64 },
    Leave { target_id: ModuleId },
    Motion { target_id: ModuleId, pos: Point64 },
    Click { target_id: ModuleId, pos: Point64, button: u32 },
    Scroll { target_id: ModuleId, axis: u32, value: f64 },
}

pub struct SignalHub {
    config: (watch::Sender<Config>, watch::Receiver<Config>),
    hyprland: (watch::Sender<HyprlandState>, watch::Receiver<HyprlandState>),
    time: (watch::Sender<chrono::DateTime<chrono::Local>>, watch::Receiver<chrono::DateTime<chrono::Local>>),
    pointer_events: broadcast::Sender<PointerEvent>,
    dirty_tx: mpsc::Sender<ModuleId>,
}

impl SignalHub {
    pub fn new(initial_config: Config) -> (Self, mpsc::Receiver<ModuleId>) {
        let config = watch::channel(initial_config);
        let hyprland = watch::channel(HyprlandState::new(Vec::new(), Vec::new()));
        let time = watch::channel(chrono::Local::now());
        let (pointer_events, _) = broadcast::channel(100);
        let (dirty_tx, dirty_rx) = mpsc::channel(100);

        (
            Self {
                config,
                hyprland,
                time,
                pointer_events,
                dirty_tx,
            },
            dirty_rx
        )
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

    pub fn dirty_tx(&self) -> mpsc::Sender<ModuleId> {
        self.dirty_tx.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::Config;

    #[tokio::test]
    async fn test_signal_hub_config_propagation() {
        let (hub, _dirty_rx) = SignalHub::new(Config::default());
        let config_rx = hub.config_rx();
        let config_tx = hub.config_tx();

        let new_config = Config::default();
        config_tx.send(new_config).unwrap();
        
        assert!(config_rx.has_changed().unwrap());
    }

    #[tokio::test]
    async fn test_signal_hub_hyprland_propagation() {
        let (hub, _dirty_rx) = SignalHub::new(Config::default());
        let hypr_rx = hub.hyprland_rx();
        let hypr_tx = hub.hyprland_tx();

        let new_state = HyprlandState::new(Vec::new(), Vec::new());
        hypr_tx.send(new_state).unwrap();

        assert!(hypr_rx.has_changed().unwrap());
    }

    #[tokio::test]
    async fn test_signal_hub_time_propagation() {
        let (hub, _dirty_rx) = SignalHub::new(Config::default());
        let time_rx = hub.time_rx();
        let time_tx = hub.time_tx();

        let now = chrono::Local::now();
        time_tx.send(now).unwrap();

        assert!(time_rx.has_changed().unwrap());
    }

    #[tokio::test]
    async fn test_signal_hub_pointer_broadcast() {
        let (hub, _dirty_rx) = SignalHub::new(Config::default());
        let mut rx1 = hub.subscribe_pointer();
        let mut rx2 = hub.subscribe_pointer();
        let tx = hub.pointer_tx();

        let event = PointerEvent::Click { target_id: ModuleId::new(1), pos: Point64::new(10.0, 10.0), button: 272 };
        tx.send(event.clone()).unwrap();

        let e1 = rx1.recv().await.unwrap();
        let e2 = rx2.recv().await.unwrap();

        match (e1, e2) {
            (PointerEvent::Click { target_id: t1, .. }, PointerEvent::Click { target_id: t2, .. }) => {
                assert_eq!(t1, ModuleId::new(1));
                assert_eq!(t2, ModuleId::new(1));
            }
            _ => panic!("Incorrect event type received"),
        }
    }

    #[tokio::test]
    async fn test_signal_hub_dirty_mpsc() {
        let (hub, mut dirty_rx) = SignalHub::new(Config::default());
        let dirty_tx = hub.dirty_tx();

        dirty_tx.send(ModuleId::new(42)).await.unwrap();
        let id = dirty_rx.recv().await.unwrap();
        assert_eq!(id, ModuleId::new(42));
    }
}
