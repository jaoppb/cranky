use crate::shared::primitives::geometry::{Scale, Size};
use crate::shared::primitives::{MonitorId, ScriptMonitorInfo};

use super::hub::SignalHub;

impl SignalHub {
    #[must_use]
    pub fn get_monitor_infos(&self) -> Vec<ScriptMonitorInfo> {
        let hypr = self.hyprland_rx();
        let hypr_guard = hypr.borrow();
        let scales = self.monitor_scales_rx();
        let scales_guard = scales.borrow();
        let focused_opt = hypr_guard.effective_focused_monitor();

        let mut infos = Vec::new();
        for (name, monitor) in hypr_guard.monitors() {
            let id_str = name.as_str().to_string();
            let mon_id = MonitorId::new(&id_str);
            let scale_val = scales_guard
                .get(&mon_id)
                .copied()
                .unwrap_or_else(|| Scale::new(1.0));
            let is_focused = focused_opt.as_ref() == Some(name);
            let active_ws = Some(monitor.active_workspace_id().value());
            let special_ws = monitor
                .special_workspace_id()
                .map(crate::features::workspaces::domain::WorkspaceId::value);

            infos.push(
                ScriptMonitorInfo::from_id(&mon_id)
                    .with_name(id_str)
                    .with_size(Size::new(0, 0))
                    .with_scale(scale_val)
                    .with_focused(is_focused)
                    .with_workspaces(active_ws, special_ws),
            );
        }
        drop(scales_guard);
        drop(hypr_guard);

        infos
    }
}
