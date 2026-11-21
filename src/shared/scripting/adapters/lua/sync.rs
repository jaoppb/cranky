//! Copies the signals the hub reports as changed into a Lua state's `signals` global.
use super::userdata::LuaMonitor;
use crate::shared::events::signals::{SignalHub, SignalKind};
use mlua::{Function, Lua, LuaSerdeExt};

pub(crate) struct LuaStateSynchronizer;

impl LuaStateSynchronizer {
    /// # Errors
    ///
    /// Returns `mlua::Error` if setting globals fails.
    pub(crate) fn sync(lua: &Lua, hub: &SignalHub, changed: &[SignalKind]) -> mlua::Result<()> {
        let globals = lua.globals();
        let signals_table = if let Some(table) = globals.get::<Option<mlua::Table>>("signals")? {
            table
        } else {
            let table = lua.create_table()?;
            globals.set("signals", table.clone())?;
            table
        };
        let mut dbus_handled = false;

        for signal in changed {
            match signal {
                SignalKind::Time => {
                    let time = hub.time_rx().borrow().to_string();
                    signals_table.set("time", time)?;
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
                    let start = std::time::Instant::now();
                    let systray = hub.systray_rx().borrow().clone();
                    let items = systray.items().values().collect::<Vec<_>>();
                    let item_count = items.len();
                    tracing::debug!(item_count, "Serializing systray to Lua");
                    match lua.to_value(&items) {
                        Ok(val) => {
                            signals_table.set("systray", val)?;
                            tracing::debug!(
                                item_count,
                                duration_ms = start.elapsed().as_millis(),
                                duration_micros = start.elapsed().as_micros(),
                                "Successfully serialized systray to Lua"
                            );
                        }
                        Err(error) => {
                            tracing::error!(err = ?error, "Failed to serialize systray to Lua");
                        }
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
        for (index, info) in monitor_infos.into_iter().enumerate() {
            if info.is_focused() {
                focused_mon = Some(LuaMonitor(info.clone()));
            }
            mon_table.set(index.saturating_add(1), LuaMonitor(info))?;
        }
        globals.set("_raw_monitors", mon_table)?;
        globals.set("_raw_focused_monitor", focused_mon)?;

        if let Ok(refresh_fn) = globals.get::<Function>("refresh") {
            let start = std::time::Instant::now();
            match refresh_fn.call::<()>(()) {
                Ok(()) => {
                    tracing::debug!(
                        duration_ms = start.elapsed().as_millis(),
                        duration_micros = start.elapsed().as_micros(),
                        "Lua refresh function called successfully"
                    );
                }
                Err(error) => {
                    tracing::error!(err = ?error, "Lua refresh function failed");
                }
            }
        }

        Ok(())
    }
}
