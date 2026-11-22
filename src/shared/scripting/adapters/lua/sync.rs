//! Copies the signals the hub reports as changed into a Lua state's `signals` global.
use super::userdata::LuaMonitor;
use crate::shared::events::signals::{SignalHub, SignalKind};
use mlua::{Function, Lua, LuaSerdeExt};
use std::time::Instant;
use tokio::sync::watch;

pub(crate) struct LuaStateSynchronizer;

impl LuaStateSynchronizer {
    /// # Errors
    ///
    /// Returns `mlua::Error` if setting globals fails.
    pub(crate) fn sync(lua: &Lua, hub: &SignalHub, changed: &[SignalKind]) -> mlua::Result<()> {
        let globals = lua.globals();
        let signals = SignalsTable {
            lua,
            table: signals_table(lua, &globals)?,
        };
        for signal in without_repeated_dbus(changed) {
            signals.publish(hub, signal)?;
        }
        publish_monitors(lua, &globals, hub)?;
        call_refresh(&globals);
        Ok(())
    }
}

/// The `signals` global of the state, created and set when the script has none.
fn signals_table(lua: &Lua, globals: &mlua::Table) -> mlua::Result<mlua::Table> {
    if let Some(table) = globals.get::<Option<mlua::Table>>("signals")? {
        return Ok(table);
    }
    let table = lua.create_table()?;
    globals.set("signals", table.clone())?;
    Ok(table)
}

/// `changed` without the `DBus` entries after the first, as one publication covers them all.
fn without_repeated_dbus(changed: &[SignalKind]) -> impl Iterator<Item = SignalKind> + '_ {
    let mut dbus_seen = false;
    changed.iter().copied().filter(move |signal| {
        *signal != SignalKind::DBus || !std::mem::replace(&mut dbus_seen, true)
    })
}

/// The current value of `receiver`, cloned so the channel is not held while it converts.
fn snapshot<T: Clone>(receiver: &watch::Receiver<T>) -> T {
    receiver.borrow().clone()
}

/// A state's `signals` table, with the state its values are created in.
struct SignalsTable<'lua> {
    lua: &'lua Lua,
    table: mlua::Table,
}

impl SignalsTable<'_> {
    /// Publishes the `signal` state of `hub` under its key. A state that does not convert is
    /// skipped, and an `mpris` state that cannot be set is ignored.
    fn publish(&self, hub: &SignalHub, signal: SignalKind) -> mlua::Result<()> {
        match signal {
            SignalKind::Time => {
                let time = hub.time_rx().borrow().to_string();
                self.table.set("time", time)
            }
            SignalKind::Hyprland => self.set_converted("hyprland", &snapshot(&hub.hyprland_rx())),
            SignalKind::DBus => self.set_converted("dbus", snapshot(&hub.dbus_rx()).properties()),
            SignalKind::Systray => self.publish_systray(hub),
            SignalKind::Metrics => self.set_converted("metrics", &snapshot(&hub.metrics_rx())),
            SignalKind::Mpris => {
                let _ = self.set_converted("mpris", &snapshot(&hub.mpris_rx()));
                Ok(())
            }
        }
    }

    /// Sets `key` to `value` converted to Lua, unless it does not convert.
    fn set_converted(&self, key: &str, value: &impl serde::Serialize) -> mlua::Result<()> {
        if let Ok(val) = self.lua.to_value(value) {
            self.table.set(key, val)?;
        }
        Ok(())
    }

    /// Publishes the tray items of `hub` as `systray`, logging how long serialising them took.
    fn publish_systray(&self, hub: &SignalHub) -> mlua::Result<()> {
        let start = Instant::now();
        let systray = snapshot(&hub.systray_rx());
        let items = systray.items().values().collect::<Vec<_>>();
        let item_count = items.len();
        log_serializing_systray(item_count);
        match self.lua.to_value(&items) {
            Ok(val) => {
                self.table.set("systray", val)?;
                log_systray_serialized(item_count, start);
            }
            Err(error) => log_systray_failed(&error),
        }
        Ok(())
    }
}

/// Publishes the monitors of `hub` as `_raw_monitors`, and the focused one, if any, as
/// `_raw_focused_monitor`.
fn publish_monitors(lua: &Lua, globals: &mlua::Table, hub: &SignalHub) -> mlua::Result<()> {
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
    globals.set("_raw_focused_monitor", focused_mon)
}

/// Runs the script's `refresh` when it defines one, logging how long it took or why it failed.
fn call_refresh(globals: &mlua::Table) {
    let Ok(refresh_fn) = globals.get::<Function>("refresh") else {
        return;
    };
    let start = Instant::now();
    match refresh_fn.call::<()>(()) {
        Ok(()) => log_refresh_called(start),
        Err(error) => log_refresh_failed(&error),
    }
}

/// Logs `item_count` tray items about to be serialised.
fn log_serializing_systray(item_count: usize) {
    tracing::debug!(item_count, "Serializing systray to Lua");
}

/// Logs `item_count` tray items serialised, with how long it took since `start`.
fn log_systray_serialized(item_count: usize, start: Instant) {
    tracing::debug!(
        item_count,
        duration_ms = start.elapsed().as_millis(),
        duration_micros = start.elapsed().as_micros(),
        "Successfully serialized systray to Lua"
    );
}

/// Logs tray items that failed to serialise with `error`.
fn log_systray_failed(error: &mlua::Error) {
    tracing::error!(err = ?error, "Failed to serialize systray to Lua");
}

/// Logs a script `refresh` that ran, with how long it took since `start`.
fn log_refresh_called(start: Instant) {
    tracing::debug!(
        duration_ms = start.elapsed().as_millis(),
        duration_micros = start.elapsed().as_micros(),
        "Lua refresh function called successfully"
    );
}

/// Logs a script `refresh` that failed with `error`.
fn log_refresh_failed(error: &mlua::Error) {
    tracing::error!(err = ?error, "Lua refresh function failed");
}
