use super::userdata::LuaMonitor;
use crate::shared::events::signals::{SignalHub, SignalKind};
use mlua::{Function, Lua, LuaSerdeExt};

pub struct LuaStateSynchronizer;

impl LuaStateSynchronizer {
    /// # Errors
    ///
    /// Returns `mlua::Error` if setting globals fails.
    pub fn sync(lua: &Lua, hub: &SignalHub, changed: &[SignalKind]) -> mlua::Result<()> {
        let globals = lua.globals();
        let signals_table = if let Some(t) = globals.get::<Option<mlua::Table>>("signals")? {
            t
        } else {
            let t = lua.create_table()?;
            globals.set("signals", t.clone())?;
            t
        };
        let mut dbus_handled = false;

        for signal in changed {
            match signal {
                SignalKind::Time => {
                    let time = *hub.time_rx().borrow();
                    signals_table.set("time", time.to_rfc3339())?;
                }
                SignalKind::Hyprland => {
                    let hypr = hub.hyprland_rx().borrow().clone();
                    if let Ok(val) = lua.to_value(&hypr) {
                        signals_table.set("hyprland", val)?;
                    }
                }
                SignalKind::DBus if !dbus_handled => {
                    let dbus_state = hub.dbus_rx().borrow().clone();
                    if let Ok(val) = lua.to_value(&dbus_state.properties()) {
                        signals_table.set("dbus", val)?;
                    }
                    dbus_handled = true;
                }
                SignalKind::Systray => {
                    let t0 = std::time::Instant::now();
                    let systray = hub.systray_rx().borrow().clone();
                    let items = systray.items().values().collect::<Vec<_>>();
                    let item_count = items.len();
                    tracing::debug!(item_count, "Serializing systray to Lua");
                    match lua.to_value(&items) {
                        Ok(val) => {
                            signals_table.set("systray", val)?;
                            tracing::debug!(
                                item_count,
                                duration_ms = t0.elapsed().as_millis(),
                                duration_micros = t0.elapsed().as_micros(),
                                "Successfully serialized systray to Lua"
                            );
                        }
                        Err(e) => tracing::error!(err = ?e, "Failed to serialize systray to Lua"),
                    }
                }
                SignalKind::Metrics => {
                    let metrics = hub.metrics_rx().borrow().clone();
                    if let Ok(val) = lua.to_value(&metrics) {
                        signals_table.set("metrics", val)?;
                    }
                }
                SignalKind::Mpris => {
                    let mpris = hub.mpris_rx().borrow().clone();
                    if let Ok(val) = lua.to_value(&mpris) {
                        let _ = signals_table.set("mpris", val);
                    }
                }
                SignalKind::DBus => {}
            }
        }

        // Update raw monitors
        let monitor_infos = hub.get_monitor_infos();
        let mon_table = lua.create_table()?;
        let mut focused_mon = None;
        for (i, info) in monitor_infos.into_iter().enumerate() {
            if info.is_focused() {
                focused_mon = Some(LuaMonitor(info.clone()));
            }
            mon_table.set(i.saturating_add(1), LuaMonitor(info))?;
        }
        globals.set("_raw_monitors", mon_table)?;
        globals.set("_raw_focused_monitor", focused_mon)?;

        if let Ok(refresh_fn) = globals.get::<Function>("refresh") {
            let t0 = std::time::Instant::now();
            match refresh_fn.call::<()>(()) {
                Ok(()) => {
                    tracing::debug!(
                        duration_ms = t0.elapsed().as_millis(),
                        duration_micros = t0.elapsed().as_micros(),
                        "Lua refresh function called successfully"
                    );
                }
                Err(e) => {
                    tracing::error!(err = ?e, "Lua refresh function failed");
                }
            }
        }

        Ok(())
    }
}
