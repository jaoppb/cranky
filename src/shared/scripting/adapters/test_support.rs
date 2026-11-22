//! Helpers the Lua and Rhai adapter tests share: a signal hub with known monitors and a marker
//! file a spawned shell command writes.
use crate::features::workspaces::domain::{Monitor, MonitorName, WorkspaceId};
use crate::shared::config::domain::Config;
use crate::shared::events::signals::{HyprlandState, SignalHub};
use crate::shared::primitives::MonitorId;
use crate::shared::primitives::geometry::Scale;
use std::collections::{BTreeMap, HashMap};

/// A hub with DP-1 at scale 2, focused and known to Hyprland on workspace 4, and DP-2 at scale 1.5.
pub(super) fn hub_with_monitors() -> SignalHub {
    let hub = SignalHub::new(Config::default());
    let monitors = BTreeMap::from([(
        MonitorName::new("DP-1"),
        Monitor::new(MonitorName::new("DP-1"), WorkspaceId::new(4), None),
    )]);
    let state = HyprlandState::new(BTreeMap::new(), monitors, Some(MonitorName::new("DP-1")));
    hub.hyprland_tx().send(state).unwrap();
    hub.monitor_scales_tx()
        .send(HashMap::from([
            (MonitorId::new("DP-1"), Scale::new(2.0)),
            (MonitorId::new("DP-2"), Scale::new(1.5)),
        ]))
        .unwrap();
    hub
}

/// A path no other test uses, for a file a spawned command creates; removed if it exists.
pub(super) fn marker_path(label: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "cranky-test-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    path
}

/// Whether `path` exists within five seconds, then removes it.
pub(super) fn file_appears(path: &std::path::Path) -> bool {
    for _ in 0..100 {
        if path.exists() {
            let _ = std::fs::remove_file(path);
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    false
}
