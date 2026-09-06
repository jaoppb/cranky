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

pub(crate) fn register_rhai_sys(engine: &mut Engine) {
    // env & hostname
    engine.register_fn("env", |_target: rhai::Map, key: String| -> Dynamic {
        std::env::var(key).map_or(Dynamic::UNIT, Dynamic::from)
    });
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

    // monitors
    engine.register_fn("all", |target: rhai::Map| -> rhai::Array {
        target
            .get("_list")
            .and_then(|v| v.clone().try_cast::<rhai::Array>())
            .unwrap_or_default()
    });
    engine.register_fn(
        "current",
        |_target: rhai::Map, cur: rhai::Map| -> rhai::Map { cur },
    );
    engine.register_fn("focused", |target: rhai::Map| -> Dynamic {
        let list = target
            .get("_list")
            .and_then(|v| v.clone().try_cast::<rhai::Array>())
            .unwrap_or_default();
        for item in list {
            let Some(map) = item.clone().try_cast::<rhai::Map>() else {
                continue;
            };
            let is_focused = map
                .get("is_focused")
                .and_then(|v| v.clone().try_cast::<bool>())
                .unwrap_or(false);
            if is_focused {
                return Dynamic::from(map);
            }
        }
        Dynamic::UNIT
    });
    engine.register_fn("by_name", |target: rhai::Map, name: String| -> Dynamic {
        let list = target
            .get("_list")
            .and_then(|v| v.clone().try_cast::<rhai::Array>())
            .unwrap_or_default();
        for item in list {
            let Some(map) = item.clone().try_cast::<rhai::Map>() else {
                continue;
            };
            let item_name = map.get("name").and_then(|v| v.clone().try_cast::<String>());
            if item_name.as_deref() == Some(name.as_str()) {
                return Dynamic::from(map);
            }
        }
        Dynamic::UNIT
    });

    // monitor to_string
    engine.register_fn("to_string", |m: rhai::Map| -> String {
        m.get("id")
            .and_then(|v| v.clone().try_cast::<String>())
            .unwrap_or_default()
    });
}
