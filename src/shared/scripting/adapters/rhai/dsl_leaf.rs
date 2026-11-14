use rhai::{Dynamic, Engine};

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

fn register_rhai_text_and_progress(engine: &mut Engine) {
    register_text_elements!(engine, String, i64, f64);

    engine.register_fn(
        "progress",
        |_target: rhai::Map, mut props: rhai::Map| -> rhai::Map {
            props.insert("type".into(), Dynamic::from("progress"));
            props
        },
    );
    engine.register_fn("progress", |_target: rhai::Map, val: f64| -> rhai::Map {
        let mut m = rhai::Map::new();
        m.insert("type".into(), Dynamic::from("progress"));
        m.insert("value".into(), Dynamic::from(val));
        m
    });
    engine.register_fn(
        "progress",
        |_target: rhai::Map, val: f64, class: String| -> rhai::Map {
            let mut m = rhai::Map::new();
            m.insert("type".into(), Dynamic::from("progress"));
            m.insert("value".into(), Dynamic::from(val));
            m.insert("class".into(), Dynamic::from(class));
            m
        },
    );
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
        m.insert(
            "value".into(),
            Dynamic::from(i32::try_from(val).map_or(0.0, f64::from)),
        );
        m
    });
    engine.register_fn(
        "progress",
        |_target: rhai::Map, val: i64, class: String| -> rhai::Map {
            let mut m = rhai::Map::new();
            m.insert("type".into(), Dynamic::from("progress"));
            m.insert(
                "value".into(),
                Dynamic::from(i32::try_from(val).map_or(0.0, f64::from)),
            );
            m.insert("class".into(), Dynamic::from(class));
            m
        },
    );
    engine.register_fn(
        "progress",
        |_target: rhai::Map, val: i64, orientation: String, class: String| -> rhai::Map {
            let mut m = rhai::Map::new();
            m.insert("type".into(), Dynamic::from("progress"));
            m.insert(
                "value".into(),
                Dynamic::from(i32::try_from(val).map_or(0.0, f64::from)),
            );
            m.insert("orientation".into(), Dynamic::from(orientation));
            m.insert("class".into(), Dynamic::from(class));
            m
        },
    );
}

fn register_rhai_rect_and_image(engine: &mut Engine) {
    engine.register_fn(
        "rect",
        |_target: rhai::Map, mut props: rhai::Map| -> rhai::Map {
            props.insert("type".into(), Dynamic::from("rect"));
            props
        },
    );
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

    engine.register_fn(
        "image",
        |_target: rhai::Map, mut props: rhai::Map| -> rhai::Map {
            props.insert("type".into(), Dynamic::from("image"));
            props
        },
    );
}

pub(super) fn register_rhai_leaf_elements(engine: &mut Engine) {
    register_rhai_text_and_progress(engine);
    register_rhai_rect_and_image(engine);
}
