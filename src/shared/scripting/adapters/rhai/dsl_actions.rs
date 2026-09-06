use rhai::{Dynamic, Engine};

macro_rules! register_script_call_aliases {
    ($engine:expr, $($name:literal),*) => {
        $(
            $engine.register_fn($name, |_target: rhai::Map, func_name: String| -> rhai::Map {
                let mut m = rhai::Map::new();
                m.insert("ScriptCall".into(), Dynamic::from(func_name));
                m
            });
        )*
    };
}

pub(crate) fn register_rhai_ui_actions(engine: &mut Engine) {
    engine.register_fn("exec", |target: rhai::Map, cmd: String| -> Dynamic {
        let is_action = target
            .get("_ns")
            .and_then(|v| v.clone().try_cast::<String>())
            .as_deref()
            == Some("ui.action");
        if is_action {
            let mut m = rhai::Map::new();
            m.insert("Exec".into(), Dynamic::from(cmd));
            return Dynamic::from(m);
        }
        let _ = std::process::Command::new("sh").arg("-c").arg(&cmd).spawn();
        Dynamic::UNIT
    });

    register_script_call_aliases!(engine, "script_call", "call_script", "func", "call");

    engine.register_fn(
        "systray",
        |_target: rhai::Map, id: String, action: String| -> rhai::Map {
            let mut inner = rhai::Map::new();
            inner.insert("id".into(), Dynamic::from(id));
            inner.insert("action".into(), Dynamic::from(action));
            let mut m = rhai::Map::new();
            m.insert("SystrayAction".into(), Dynamic::from(inner));
            m
        },
    );

    engine.register_fn("systray", |_target: rhai::Map, id: String| -> rhai::Map {
        let mut inner = rhai::Map::new();
        inner.insert("id".into(), Dynamic::from(id));
        inner.insert("action".into(), Dynamic::from("Primary".to_string()));
        let mut m = rhai::Map::new();
        m.insert("SystrayAction".into(), Dynamic::from(inner));
        m
    });

    engine.register_fn(
        "systray",
        |_target: rhai::Map, id: i64, action: String| -> rhai::Map {
            let mut inner = rhai::Map::new();
            inner.insert("id".into(), Dynamic::from(id.to_string()));
            inner.insert("action".into(), Dynamic::from(action));
            let mut m = rhai::Map::new();
            m.insert("SystrayAction".into(), Dynamic::from(inner));
            m
        },
    );

    engine.register_fn("systray", |_target: rhai::Map, id: i64| -> rhai::Map {
        let mut inner = rhai::Map::new();
        inner.insert("id".into(), Dynamic::from(id.to_string()));
        inner.insert("action".into(), Dynamic::from("Primary".to_string()));
        let mut m = rhai::Map::new();
        m.insert("SystrayAction".into(), Dynamic::from(inner));
        m
    });
}
