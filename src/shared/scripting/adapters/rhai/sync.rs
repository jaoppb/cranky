use super::helpers::monitor_info_to_rhai_map;
use super::script::RhaiScript;
use crate::shared::events::signals::{SignalHub, SignalKind};
use crate::shared::primitives::ScriptMonitorInfo;
use rhai::{Dynamic, Engine};
use tokio::sync::watch;

/// The signals a refresh can publish, in the order it publishes them; each one changed is
/// published once.
const PUBLISHED_SIGNALS: [SignalKind; 6] = [
    SignalKind::Time,
    SignalKind::Hyprland,
    SignalKind::Systray,
    SignalKind::Metrics,
    SignalKind::Mpris,
    SignalKind::DBus,
];

pub(super) struct RhaiStateSynchronizer;

impl RhaiStateSynchronizer {
    /// Publishes the `changed` signals and the monitors to the script, runs its `refresh`, and
    /// returns the monitors it was given.
    pub(crate) fn sync(
        script: &mut RhaiScript<'_>,
        hub: &SignalHub,
        changed: &[SignalKind],
    ) -> Vec<ScriptMonitorInfo> {
        let signals_map = publish_signals(script, hub, changed);
        script.scope_mut().set_value("signals", signals_map.clone());
        let monitor_infos = hub.get_monitor_infos();
        let sys_map = publish_monitors(script, &monitor_infos);
        publish_to_cranky(script, signals_map, sys_map);
        call_refresh(script);
        monitor_infos
    }
}

/// The script's `signals` map with each `changed` signal of `hub` published into it.
fn publish_signals(
    script: &mut RhaiScript<'_>,
    hub: &SignalHub,
    changed: &[SignalKind],
) -> rhai::Map {
    let map = script
        .scope_mut()
        .get_value::<rhai::Map>("signals")
        .unwrap_or_default();
    let mut signals = SignalsMap {
        engine: script.engine(),
        map,
    };
    for signal in PUBLISHED_SIGNALS {
        if changed.contains(&signal) {
            signals.publish(hub, signal);
        }
    }
    signals.map
}

/// The current value of `receiver`, cloned so the channel is not held while it converts.
fn snapshot<T: Clone>(receiver: &watch::Receiver<T>) -> T {
    receiver.borrow().clone()
}

/// A script's `signals` map, with the engine its values are parsed by.
struct SignalsMap<'engine> {
    engine: &'engine Engine,
    map: rhai::Map,
}

impl SignalsMap<'_> {
    /// Publishes the `signal` state of `hub` under its key; a state that does not convert is
    /// skipped.
    fn publish(&mut self, hub: &SignalHub, signal: SignalKind) {
        match signal {
            SignalKind::Time => {
                let time = hub.time_rx().borrow().to_string();
                self.map.insert("time".into(), Dynamic::from(time));
            }
            SignalKind::Hyprland => {
                self.insert_converted("hyprland", &snapshot(&hub.hyprland_rx()));
            }
            SignalKind::DBus => {
                self.insert_converted("dbus", snapshot(&hub.dbus_rx()).properties());
            }
            SignalKind::Systray => {
                let systray = snapshot(&hub.systray_rx());
                self.insert_converted("systray", &systray.items().values().collect::<Vec<_>>());
            }
            SignalKind::Metrics => self.insert_converted("metrics", &snapshot(&hub.metrics_rx())),
            SignalKind::Mpris => self.insert_converted("mpris", &snapshot(&hub.mpris_rx())),
        }
    }

    /// Inserts `value` under `key` as the map its JSON parses to, unless it does not parse to
    /// one.
    fn insert_converted(&mut self, key: &str, value: &impl serde::Serialize) {
        if let Ok(json) = serde_json::to_string(value)
            && let Ok(converted) = self.engine.parse_json(&json, true)
        {
            self.map.insert(key.into(), Dynamic::from(converted));
        }
    }
}

/// Publishes `monitor_infos` as `sys.monitors._list`, keeping the rest of `sys`, and returns
/// the `sys` map.
fn publish_monitors(script: &mut RhaiScript<'_>, monitor_infos: &[ScriptMonitorInfo]) -> rhai::Map {
    let monitors_arr: rhai::Array = monitor_infos
        .iter()
        .map(monitor_info_to_rhai_map)
        .map(Dynamic::from)
        .collect();

    let mut sys_map = script
        .scope_mut()
        .get_value::<rhai::Map>("sys")
        .unwrap_or_default();
    let mut monitors_map = sys_map
        .get("monitors")
        .and_then(|value| value.clone().try_cast::<rhai::Map>())
        .unwrap_or_default();
    monitors_map.insert("_list".into(), Dynamic::from(monitors_arr));
    sys_map.insert("monitors".into(), Dynamic::from(monitors_map));
    script.scope_mut().set_value("sys", sys_map.clone());
    sys_map
}

/// Mirrors `signals_map` and `sys_map` into the script's `cranky` map, when it has one.
fn publish_to_cranky(script: &mut RhaiScript<'_>, signals_map: rhai::Map, sys_map: rhai::Map) {
    if let Some(mut cranky_map) = script.scope_mut().get_value::<rhai::Map>("cranky") {
        cranky_map.insert("signals".into(), Dynamic::from(signals_map));
        cranky_map.insert("sys".into(), Dynamic::from(sys_map));
        script.scope_mut().set_value("cranky", cranky_map);
    }
}

/// Runs the script's `refresh`, logging why when it fails.
fn call_refresh(script: &mut RhaiScript<'_>) {
    let (engine, scope, ast) = script.parts();
    if let Err(error) = engine.call_fn::<()>(scope, ast, "refresh", ()) {
        log_refresh_failed(&error);
    }
}

/// Logs a script `refresh` that failed with `error`.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_refresh_failed(error: &rhai::EvalAltResult) {
    tracing::error!("Rhai refresh error: {error}");
}
