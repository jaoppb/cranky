//! What `LuaStateSynchronizer::sync` publishes to a Lua state: the changed signals, the monitors,
//! and the script's `refresh` run after them.
use super::support::api_state;
use crate::shared::config::domain::Config;
use crate::shared::events::signals::{SignalHub, SignalKind};
use crate::shared::scripting::adapters::lua::sync::LuaStateSynchronizer;
use crate::shared::scripting::adapters::test_support::hub_with_monitors;
use mlua::Lua;

/// A hub whose Hyprland focus, MPRIS player and `DBus` properties are set.
fn hub_with_signals() -> SignalHub {
    let hub = SignalHub::new(Config::default());
    hub.hyprland_tx()
        .send(crate::shared::events::signals::HyprlandState::new(
            std::collections::BTreeMap::new(),
            std::collections::BTreeMap::new(),
            Some(crate::features::workspaces::domain::MonitorName::new(
                "DP-7",
            )),
        ))
        .unwrap();
    hub.mpris_tx()
        .send(crate::features::mpris::domain::MprisState {
            active_player: Some(crate::features::mpris::domain::PlayerName::new("spotify")),
            players: std::collections::HashMap::new(),
        })
        .unwrap();
    hub.dbus_tx()
        .send(crate::shared::dbus::domain::DBusState::new(
            std::collections::HashMap::from([(
                "org.example.Volume".to_string(),
                crate::shared::dbus::domain::DBusValue::Int(42),
            )]),
        ))
        .unwrap();
    hub
}

/// The string the Lua chunk `script` returns.
fn eval_string(lua: &Lua, script: &str) -> String {
    lua.load(script).eval::<String>().unwrap()
}

const SIGNALS_SUMMARY: &str = r#"
    return tostring(signals.time ~= nil) .. "|" .. tostring(signals.hyprland and signals.hyprland.focused_monitor)
        .. "|" .. type(signals.systray) .. "|" .. type(signals.metrics) .. "|"
        .. tostring(signals.mpris and signals.mpris.active_player) .. "|"
        .. tostring(signals.dbus and signals.dbus["org.example.Volume"])
"#;

#[test]
fn test_lua_sync_publishes_every_changed_signal() {
    let lua = api_state();
    let changed = [
        SignalKind::Time,
        SignalKind::Hyprland,
        SignalKind::Systray,
        SignalKind::Metrics,
        SignalKind::Mpris,
        SignalKind::DBus,
        SignalKind::DBus,
    ];
    LuaStateSynchronizer::sync(&lua, &hub_with_signals(), &changed).unwrap();
    assert_eq!(
        eval_string(&lua, SIGNALS_SUMMARY),
        "true|DP-7|table|table|spotify|42"
    );
}

#[test]
fn test_lua_sync_publishes_only_the_changed_signals() {
    let lua = api_state();
    LuaStateSynchronizer::sync(&lua, &hub_with_signals(), &[SignalKind::Mpris]).unwrap();
    assert_eq!(
        eval_string(&lua, SIGNALS_SUMMARY),
        "false|nil|nil|nil|spotify|nil"
    );
}

#[test]
fn test_lua_sync_keeps_the_signals_table_or_creates_one() {
    let lua = api_state();
    lua.load(r#"signals.custom = "kept""#).exec().unwrap();
    LuaStateSynchronizer::sync(&lua, &hub_with_signals(), &[SignalKind::Mpris]).unwrap();
    assert_eq!(
        eval_string(
            &lua,
            "return signals.custom .. cranky.signals.mpris.active_player"
        ),
        "keptspotify"
    );

    lua.load("signals = nil").exec().unwrap();
    LuaStateSynchronizer::sync(&lua, &hub_with_signals(), &[SignalKind::Mpris]).unwrap();
    assert_eq!(
        eval_string(&lua, "return signals.mpris.active_player"),
        "spotify"
    );
}

#[test]
fn test_lua_sync_publishes_the_monitors_and_the_focused_one() {
    let lua = api_state();
    LuaStateSynchronizer::sync(&lua, &hub_with_monitors(), &[]).unwrap();
    let script = r#"
        local ids = {}
        for index, monitor in ipairs(_raw_monitors) do ids[index] = monitor.id end
        table.sort(ids)
        return table.concat(ids, ",") .. "|" .. _raw_focused_monitor.id
    "#;
    assert_eq!(eval_string(&lua, script), "DP-1,DP-2|DP-1");

    let unfocused = SignalHub::new(Config::default());
    LuaStateSynchronizer::sync(&lua, &unfocused, &[]).unwrap();
    let script = r"return #_raw_monitors .. tostring(_raw_focused_monitor)";
    assert_eq!(eval_string(&lua, script), "0nil");
}

#[test]
fn test_lua_sync_runs_refresh_after_publishing_and_survives_its_error() {
    let lua = api_state();
    lua.load("function refresh() seen = signals.mpris.active_player .. #_raw_monitors end")
        .exec()
        .unwrap();
    LuaStateSynchronizer::sync(&lua, &hub_with_signals(), &[SignalKind::Mpris]).unwrap();
    assert_eq!(eval_string(&lua, "return seen"), "spotify0");

    lua.load(r#"function refresh() error("boom") end"#)
        .exec()
        .unwrap();
    assert!(LuaStateSynchronizer::sync(&lua, &hub_with_signals(), &[SignalKind::Mpris]).is_ok());
}

/// Makes every new key on `signals` an error, then syncs `changed` and reports whether
/// `refresh` ran.
fn sync_into_a_read_only_signals_table(changed: &[SignalKind]) -> (bool, bool) {
    let lua = api_state();
    lua.load(
        r#"setmetatable(signals, { __newindex = function() error("read only") end })
           ran = false
           function refresh() ran = true end"#,
    )
    .exec()
    .unwrap();
    let result = LuaStateSynchronizer::sync(&lua, &hub_with_signals(), changed);
    (result.is_ok(), lua.globals().get::<bool>("ran").unwrap())
}

#[test]
fn test_lua_sync_stops_at_a_failed_signal_except_mpris() {
    for kind in [
        SignalKind::Time,
        SignalKind::Hyprland,
        SignalKind::Systray,
        SignalKind::Metrics,
        SignalKind::DBus,
    ] {
        assert_eq!(
            sync_into_a_read_only_signals_table(&[kind]),
            (false, false),
            "{kind:?}"
        );
    }
    assert_eq!(
        sync_into_a_read_only_signals_table(&[SignalKind::Mpris]),
        (true, true)
    );
}
