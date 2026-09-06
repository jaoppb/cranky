use super::helpers::monitor_info_to_rhai_map;
use crate::shared::events::signals::{SignalHub, SignalKind};
use crate::shared::primitives::ScriptMonitorInfo;
use rhai::{AST, Dynamic, Engine, Scope};

pub(crate) struct RhaiStateSynchronizer;

impl RhaiStateSynchronizer {
    pub(crate) fn sync(
        scope: &mut Scope<'static>,
        engine: &Engine,
        ast: &AST,
        hub: &SignalHub,
        changed: &[SignalKind],
        cached_monitors: &mut Vec<ScriptMonitorInfo>,
    ) {
        let mut signals_map = scope.get_value::<rhai::Map>("signals").unwrap_or_default();

        if changed.contains(&SignalKind::Time) {
            let time = *hub.time_rx().borrow();
            signals_map.insert("time".into(), Dynamic::from(time.to_rfc3339()));
        }

        if changed.contains(&SignalKind::Hyprland) {
            let hypr = hub.hyprland_rx().borrow().clone();
            if let Ok(hypr_json) = serde_json::to_string(&hypr)
                && let Ok(hypr_rhai) = engine.parse_json(&hypr_json, true)
            {
                signals_map.insert("hyprland".into(), Dynamic::from(hypr_rhai));
            }
        }

        if changed.contains(&SignalKind::Systray) {
            let systray = hub.systray_rx().borrow().clone();
            let items = systray.items().values().collect::<Vec<_>>();
            if let Ok(systray_json) = serde_json::to_string(&items)
                && let Ok(systray_rhai) = engine.parse_json(&systray_json, true)
            {
                signals_map.insert("systray".into(), Dynamic::from(systray_rhai));
            }
        }

        if changed.contains(&SignalKind::Metrics) {
            let metrics = hub.metrics_rx().borrow().clone();
            if let Ok(metrics_json) = serde_json::to_string(&metrics)
                && let Ok(metrics_rhai) = engine.parse_json(&metrics_json, true)
            {
                signals_map.insert("metrics".into(), Dynamic::from(metrics_rhai));
            }
        }

        if changed.contains(&SignalKind::Mpris) {
            let mpris = hub.mpris_rx().borrow().clone();
            if let Ok(mpris_json) = serde_json::to_string(&mpris)
                && let Ok(mpris_rhai) = engine.parse_json(&mpris_json, true)
            {
                signals_map.insert("mpris".into(), Dynamic::from(mpris_rhai));
            }
        }

        let mut dbus_handled = false;
        for signal in changed {
            if matches!(signal, SignalKind::DBus) && !dbus_handled {
                let dbus_state = hub.dbus_rx().borrow().clone();
                if let Ok(dbus_json) = serde_json::to_string(&dbus_state.properties())
                    && let Ok(dbus_rhai) = engine.parse_json(&dbus_json, true)
                {
                    signals_map.insert("dbus".into(), Dynamic::from(dbus_rhai));
                }
                dbus_handled = true;
            }
        }

        scope.set_value("signals", signals_map.clone());

        // Update sys.monitors list in scope
        let monitor_infos = hub.get_monitor_infos();
        cached_monitors.clone_from(&monitor_infos);
        let monitors_arr: rhai::Array = monitor_infos
            .iter()
            .map(monitor_info_to_rhai_map)
            .map(Dynamic::from)
            .collect();

        let mut sys_map = scope.get_value::<rhai::Map>("sys").unwrap_or_default();
        let mut monitors_map = sys_map
            .get("monitors")
            .and_then(|v| v.clone().try_cast::<rhai::Map>())
            .unwrap_or_default();
        monitors_map.insert("_list".into(), Dynamic::from(monitors_arr));
        sys_map.insert("monitors".into(), Dynamic::from(monitors_map));
        scope.set_value("sys", sys_map.clone());

        // Update cranky map
        if let Some(mut cranky_map) = scope.get_value::<rhai::Map>("cranky") {
            cranky_map.insert("signals".into(), Dynamic::from(signals_map));
            cranky_map.insert("sys".into(), Dynamic::from(sys_map));
            scope.set_value("cranky", cranky_map);
        }

        if let Err(e) = engine.call_fn::<()>(scope, ast, "refresh", ()) {
            tracing::error!("Rhai refresh error: {e}");
        }
    }
}
