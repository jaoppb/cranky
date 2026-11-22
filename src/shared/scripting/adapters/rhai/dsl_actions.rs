use rhai::{Dynamic, Engine};

macro_rules! register_script_call_aliases {
    ($engine:expr, $($name:literal),*) => {
        $(
            $engine.register_fn($name, |_target: rhai::Map, func_name: String| -> rhai::Map {
                let mut action_map = rhai::Map::new();
                action_map.insert("ScriptCall".into(), Dynamic::from(func_name));
                action_map
            });
        )*
    };
}

pub(super) fn register_rhai_ui_actions(engine: &mut Engine) {
    engine.register_fn("exec", |target: rhai::Map, cmd: String| {
        exec_action_or_command(&target, cmd)
    });

    register_script_call_aliases!(engine, "script_call", "call_script", "func", "call");

    register_rhai_systray_actions(engine);
}

/// `ui.action.exec(cmd)`: the action running `cmd`. On any other `target`, such as `sys`, `cmd`
/// starts at once with `sh -c`, a failed start ignored, and the call returns unit.
fn exec_action_or_command(target: &rhai::Map, cmd: String) -> Dynamic {
    let is_action = target
        .get("_ns")
        .and_then(|value| value.clone().try_cast::<String>())
        .as_deref()
        == Some("ui.action");
    if is_action {
        let mut action_map = rhai::Map::new();
        action_map.insert("Exec".into(), Dynamic::from(cmd));
        return Dynamic::from(action_map);
    }
    run_shell_command(&cmd);
    Dynamic::UNIT
}

/// Starts `cmd` with `sh -c` without waiting for it; a failed start is ignored.
pub(super) fn run_shell_command(cmd: &str) {
    let _ = std::process::Command::new("sh").arg("-c").arg(cmd).spawn();
}

/// Registers `ui.action.systray(id)` and `ui.action.systray(id, action)`, with a string or
/// integer `id` and `"Primary"` when no action is given.
fn register_rhai_systray_actions(engine: &mut Engine) {
    engine.register_fn(
        "systray",
        |_target: rhai::Map, id: String, action: String| systray_action(id, action),
    );
    engine.register_fn("systray", |_target: rhai::Map, id: String| {
        systray_action(id, "Primary".to_string())
    });
    engine.register_fn("systray", |_target: rhai::Map, id: i64, action: String| {
        systray_action(id.to_string(), action)
    });
    engine.register_fn("systray", |_target: rhai::Map, id: i64| {
        systray_action(id.to_string(), "Primary".to_string())
    });
}

/// The action asking tray item `id` for `action`.
fn systray_action(id: String, action: String) -> rhai::Map {
    let mut inner = rhai::Map::new();
    inner.insert("id".into(), Dynamic::from(id));
    inner.insert("action".into(), Dynamic::from(action));
    let mut action_map = rhai::Map::new();
    action_map.insert("SystrayAction".into(), Dynamic::from(inner));
    action_map
}
