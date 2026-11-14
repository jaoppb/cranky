use super::dsl_leaf::register_rhai_leaf_elements;
use rhai::{Dynamic, Engine};

macro_rules! register_container_elements {
    ($engine:expr, $($name:literal),*) => {
        $(
            $engine.register_fn($name, |_target: rhai::Map, children: rhai::Array| -> rhai::Map {
                let mut m = rhai::Map::new();
                m.insert("type".into(), Dynamic::from($name));
                m.insert("children".into(), Dynamic::from(children));
                m
            });
            $engine.register_fn($name, |_target: rhai::Map, mut props: rhai::Map| -> rhai::Map {
                props.insert("type".into(), Dynamic::from($name));
                props
            });
            $engine.register_fn($name, |_target: rhai::Map| -> rhai::Map {
                let mut m = rhai::Map::new();
                m.insert("type".into(), Dynamic::from($name));
                m.insert("children".into(), Dynamic::from(rhai::Array::new()));
                m
            });
        )*
    };
}

fn register_rhai_flex_and_grid(engine: &mut Engine) {
    register_container_elements!(engine, "flex", "grid");
}

fn register_rhai_module_and_floating(engine: &mut Engine) {
    for fn_name in ["module", "widget", "load_module", "mod_element"] {
        engine.register_fn(
            fn_name,
            |_target: rhai::Map, mut props: rhai::Map| -> rhai::Map {
                props.insert("type".into(), Dynamic::from("module"));
                props
            },
        );
        engine.register_fn(fn_name, |_target: rhai::Map, name: String| -> rhai::Map {
            let mut m = rhai::Map::new();
            m.insert("type".into(), Dynamic::from("module"));
            m.insert("name".into(), Dynamic::from(name));
            m
        });
        engine.register_fn(
            fn_name,
            |_target: rhai::Map, name: String, class: String| -> rhai::Map {
                let mut m = rhai::Map::new();
                m.insert("type".into(), Dynamic::from("module"));
                m.insert("name".into(), Dynamic::from(name));
                m.insert("class".into(), Dynamic::from(class));
                m
            },
        );
    }

    engine.register_fn(
        "popup",
        |_target: rhai::Map, mut props: rhai::Map| -> rhai::Map {
            props.insert("type".into(), Dynamic::from("popup"));
            props
        },
    );
    engine.register_fn(
        "panel",
        |_target: rhai::Map, mut props: rhai::Map| -> rhai::Map {
            props.insert("type".into(), Dynamic::from("panel"));
            props
        },
    );
    engine.register_fn("popup", |mut props: rhai::Map| -> rhai::Map {
        props.insert("type".into(), Dynamic::from("popup"));
        props
    });
    engine.register_fn("panel", |mut props: rhai::Map| -> rhai::Map {
        props.insert("type".into(), Dynamic::from("panel"));
        props
    });
}

pub(crate) fn register_rhai_ui_elements(engine: &mut Engine) {
    register_rhai_flex_and_grid(engine);
    register_rhai_leaf_elements(engine);
    register_rhai_module_and_floating(engine);
}
