use crate::features::workspaces::domain::MonitorName;
use crate::shared::primitives::ScriptMonitorInfo;
use crate::shared::primitives::geometry::Size;

use super::hub::SignalHub;

impl SignalHub {
    /// The script-facing monitor list. Iterates Wayland's monitor set (the
    /// only subsystem that knows whether a surface can exist at all) and
    /// enriches each entry with Hyprland's workspace data when available -
    /// a monitor Wayland knows about but Hyprland hasn't announced yet
    /// (see `monitoraddedv2`'s lag) still appears, just without workspace
    /// ids, rather than being silently absent from the list `render()` is
    /// being called for.
    #[must_use]
    pub fn get_monitor_infos(&self) -> Vec<ScriptMonitorInfo> {
        let hypr = self.hyprland_rx();
        let hypr_guard = hypr.borrow();
        let scales = self.monitor_scales_rx();
        let scales_guard = scales.borrow();
        let focused_opt = hypr_guard.effective_focused_monitor();

        let mut infos = Vec::new();
        for (mon_id, scale) in scales_guard.iter() {
            let hypr_monitor = hypr_guard
                .monitors()
                .get(&MonitorName::new(mon_id.as_str()));
            let is_focused = hypr_monitor.is_some()
                && focused_opt.as_ref().map(MonitorName::as_str) == Some(mon_id.as_str());
            let (active_ws, special_ws) = hypr_monitor.map_or((None, None), |monitor| {
                (
                    Some(monitor.active_workspace_id().value()),
                    monitor
                        .special_workspace_id()
                        .map(crate::features::workspaces::domain::WorkspaceId::value),
                )
            });

            infos.push(
                ScriptMonitorInfo::from_id(mon_id)
                    .with_name(mon_id.as_str().to_string())
                    .with_size(Size::new(0, 0))
                    .with_scale(*scale)
                    .with_focused(is_focused)
                    .with_workspaces(active_ws, special_ws),
            );
        }
        drop(scales_guard);
        drop(hypr_guard);

        infos
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::workspaces::domain::{Monitor, WorkspaceId};
    use crate::shared::config::domain::Config;
    use crate::shared::events::signals::HyprlandState;
    use crate::shared::primitives::geometry::Scale;
    use std::collections::BTreeMap;

    #[test]
    fn test_get_monitor_infos_enriches_from_hyprland_when_known() {
        let hub = SignalHub::new(Config::default());

        let mut monitors = BTreeMap::new();
        monitors.insert(
            MonitorName::new("DP-1"),
            Monitor::new(MonitorName::new("DP-1"), WorkspaceId::new(4), None),
        );
        let h_state = HyprlandState::new(BTreeMap::new(), monitors, Some(MonitorName::new("DP-1")));
        hub.hyprland_tx().send(h_state).unwrap();

        let mut scales = hub.monitor_scales_rx().borrow().clone();
        scales.insert(
            crate::shared::primitives::MonitorId::new("DP-1"),
            Scale::new(2.0),
        );
        hub.monitor_scales_tx().send(scales).unwrap();

        let infos = hub.get_monitor_infos();
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].name(), "DP-1");
        assert_eq!(infos[0].scale(), Scale::new(2.0));
        assert!(infos[0].is_focused());
        assert_eq!(infos[0].active_workspace_id(), Some(4));
    }

    #[test]
    fn test_get_monitor_infos_includes_monitor_unknown_to_hyprland() {
        let hub = SignalHub::new(Config::default());

        // Wayland knows about HDMI-A-1; Hyprland hasn't announced it yet
        // (monitoraddedv2 lag).
        let mut scales = hub.monitor_scales_rx().borrow().clone();
        scales.insert(
            crate::shared::primitives::MonitorId::new("HDMI-A-1"),
            Scale::new(1.0),
        );
        hub.monitor_scales_tx().send(scales).unwrap();

        let infos = hub.get_monitor_infos();
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].name(), "HDMI-A-1");
        assert!(!infos[0].is_focused());
        assert_eq!(infos[0].active_workspace_id(), None);
        assert_eq!(infos[0].special_workspace_id(), None);
    }

    #[test]
    fn test_get_monitor_infos_excludes_monitor_unknown_to_wayland() {
        let hub = SignalHub::new(Config::default());

        // Hyprland reports a monitor Wayland has no surface for at all.
        let mut monitors = BTreeMap::new();
        monitors.insert(
            MonitorName::new("GHOST-1"),
            Monitor::new(MonitorName::new("GHOST-1"), WorkspaceId::new(1), None),
        );
        let h_state = HyprlandState::new(BTreeMap::new(), monitors, None);
        hub.hyprland_tx().send(h_state).unwrap();

        let infos = hub.get_monitor_infos();
        assert!(infos.is_empty());
    }
}
