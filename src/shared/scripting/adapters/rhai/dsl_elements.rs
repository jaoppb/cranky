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

macro_rules! register_text_elements {
    ($engine:expr, $($t:ty),*) => {
        $engine.register_fn("text", |_target: rhai::Map, mut props: rhai::Map| -> rhai::Map {
            props.insert("type".into(), Dynamic::from("text"));
            props
        });
        $(
            $engine.register_fn("text", |_target: rhai::Map, val: $t| -> rhai::Map {
                let mut m = rhai::Map::new();
                m.insert("type".into(), Dynamic::from("text"));
                m.insert("text".into(), Dynamic::from(val.to_string()));
                m
            });
            $engine.register_fn("text", |_target: rhai::Map, val: $t, class: String| -> rhai::Map {
                let mut m = rhai::Map::new();
                m.insert("type".into(), Dynamic::from("text"));
                m.insert("text".into(), Dynamic::from(val.to_string()));
                m.insert("class".into(), Dynamic::from(class));
                m
            });
        )*
    };
}

fn register_rhai_flex_and_grid(engine: &mut Engine) {
    register_container_elements!(engine, "flex", "grid");
}

fn register_rhai_text_and_progress(engine: &mut Engine) {
    register_text_elements!(engine, String, i64, f64);

    engine.register_fn("progress", |_target: rhai::Map, mut props: rhai::Map| -> rhai::Map {
        props.insert("type".into(), Dynamic::from("progress"));
        props
    });
    engine.register_fn("progress", |_target: rhai::Map, val: f64| -> rhai::Map {
        let mut m = rhai::Map::new();
        m.insert("type".into(), Dynamic::from("progress"));
        m.insert("value".into(), Dynamic::from(val));
        m
    });
    engine.register_fn("progress", |_target: rhai::Map, val: f64, class: String| -> rhai::Map {
        let mut m = rhai::Map::new();
        m.insert("type".into(), Dynamic::from("progress"));
        m.insert("value".into(), Dynamic::from(val));
        m.insert("class".into(), Dynamic::from(class));
        m
    });
    engine.register_fn(
        "progress",
        |_target: rhai::Map, val: f64, orientation: String, class: String| -> rhai::Map {
            let mut m = rhai::Map::new();
            m.insert("type".into(), Dynamic::from("progress"));
            m.insert("value".into(), Dynamic::from(val));
            m.insert("orientation".into(), Dynamic::from(orientation));
            m.insert("class".into(), Dynamic::from(class));
            m
        },
    );
    engine.register_fn("progress", |_target: rhai::Map, val: i64| -> rhai::Map {
        let mut m = rhai::Map::new();
        m.insert("type".into(), Dynamic::from("progress"));
        #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
        m.insert("value".into(), Dynamic::from(val as f64));
        m
    });
    engine.register_fn("progress", |_target: rhai::Map, val: i64, class: String| -> rhai::Map {
        let mut m = rhai::Map::new();
        m.insert("type".into(), Dynamic::from("progress"));
        #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
        m.insert("value".into(), Dynamic::from(val as f64));
        m.insert("class".into(), Dynamic::from(class));
        m
    });
    engine.register_fn(
        "progress",
        |_target: rhai::Map, val: i64, orientation: String, class: String| -> rhai::Map {
            let mut m = rhai::Map::new();
            m.insert("type".into(), Dynamic::from("progress"));
            #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
            m.insert("value".into(), Dynamic::from(val as f64));
            m.insert("orientation".into(), Dynamic::from(orientation));
            m.insert("class".into(), Dynamic::from(class));
            m
        },
    );
}

fn register_rhai_rect_image_module(engine: &mut Engine) {
    engine.register_fn("rect", |_target: rhai::Map, mut props: rhai::Map| -> rhai::Map {
        props.insert("type".into(), Dynamic::from("rect"));
        props
    });
    engine.register_fn("rect", |_target: rhai::Map, class: String| -> rhai::Map {
        let mut m = rhai::Map::new();
        m.insert("type".into(), Dynamic::from("rect"));
        m.insert("class".into(), Dynamic::from(class));
        m
    });
    engine.register_fn("rect", |_target: rhai::Map| -> rhai::Map {
        let mut m = rhai::Map::new();
        m.insert("type".into(), Dynamic::from("rect"));
        m
    });

    engine.register_fn("image", |_target: rhai::Map, mut props: rhai::Map| -> rhai::Map {
        props.insert("type".into(), Dynamic::from("image"));
        props
    });

    for fn_name in ["module", "widget", "load_module", "mod_element"] {
        engine.register_fn(fn_name, |_target: rhai::Map, mut props: rhai::Map| -> rhai::Map {
            props.insert("type".into(), Dynamic::from("module"));
            props
        });
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

    engine.register_fn("popup", |_target: rhai::Map, mut props: rhai::Map| -> rhai::Map {
        props.insert("type".into(), Dynamic::from("popup"));
        props
    });
    engine.register_fn("panel", |_target: rhai::Map, mut props: rhai::Map| -> rhai::Map {
        props.insert("type".into(), Dynamic::from("panel"));
        props
    });
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
    register_rhai_text_and_progress(engine);
    register_rhai_rect_image_module(engine);
}
