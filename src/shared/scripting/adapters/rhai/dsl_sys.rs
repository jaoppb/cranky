use super::helpers::get_hostname;
use rhai::{Dynamic, Engine};

macro_rules! register_rhai_log_methods {
    ($engine:expr, $(($name:literal, $level:ident)),*) => {
        $(
            $engine.register_fn($name, |_target: rhai::Map, msg: String| {
                tracing::$level!("{msg}");
            });
        )*
    };
}

pub(super) fn register_rhai_sys(engine: &mut Engine) {
    // env & hostname
    engine.register_fn("env", |_target: rhai::Map, key: String| env_var(&key));
    engine.register_fn("hostname", |_target: rhai::Map| -> String {
        get_hostname()
    });

    // log
    register_rhai_log_methods!(
        engine,
        ("info", info),
        ("warn", warn),
        ("error", error),
        ("debug", debug)
    );

    register_rhai_monitors(engine);

    // monitor to_string
    engine.register_fn("to_string", |monitor: rhai::Map| -> String {
        monitor
            .get("id")
            .and_then(|value| value.clone().try_cast::<String>())
            .unwrap_or_default()
    });
}

/// `sys.env(key)`: the environment variable `key`, or unit when unset.
fn env_var(key: &str) -> Dynamic {
    std::env::var(key).map_or(Dynamic::UNIT, Dynamic::from)
}

/// Registers the `sys.monitors` lookups `all`, `current`, `focused` and `by_name`.
fn register_rhai_monitors(engine: &mut Engine) {
    engine.register_fn("all", |target: rhai::Map| all_monitors(&target));
    engine.register_fn(
        "current",
        |_target: rhai::Map, cur: rhai::Map| -> rhai::Map { cur },
    );
    engine.register_fn("focused", |target: rhai::Map| focused_monitor(&target));
    engine.register_fn("by_name", |target: rhai::Map, name: String| {
        monitor_by_name(&target, &name)
    });
}

/// `sys.monitors.all()`: the monitors the last refresh listed, empty before it.
fn all_monitors(target: &rhai::Map) -> rhai::Array {
    target
        .get("_list")
        .and_then(|value| value.clone().try_cast::<rhai::Array>())
        .unwrap_or_default()
}

/// `sys.monitors.focused()`: the first listed monitor that is focused, or unit.
fn focused_monitor(target: &rhai::Map) -> Dynamic {
    all_monitors(target)
        .into_iter()
        .filter_map(Dynamic::try_cast::<rhai::Map>)
        .find(|map| {
            map.get("is_focused")
                .and_then(|value| value.clone().try_cast::<bool>())
                .unwrap_or(false)
        })
        .map_or(Dynamic::UNIT, Dynamic::from)
}

/// `sys.monitors.by_name(name)`: the first listed monitor named `name`, or unit.
fn monitor_by_name(target: &rhai::Map, name: &str) -> Dynamic {
    all_monitors(target)
        .into_iter()
        .filter_map(Dynamic::try_cast::<rhai::Map>)
        .find(|map| {
            map.get("name")
                .and_then(|value| value.clone().try_cast::<String>())
                .as_deref()
                == Some(name)
        })
        .map_or(Dynamic::UNIT, Dynamic::from)
}
