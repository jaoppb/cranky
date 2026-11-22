use super::helpers::with_type;
use rhai::{Dynamic, Engine};

macro_rules! register_text_elements {
    ($engine:expr, $($t:ty),*) => {
        $engine.register_fn("text", |_target: rhai::Map, mut props: rhai::Map| -> rhai::Map {
            props.insert("type".into(), Dynamic::from("text"));
            props
        });
        $(
            $engine.register_fn("text", |_target: rhai::Map, val: $t| -> rhai::Map {
                let mut element = rhai::Map::new();
                element.insert("type".into(), Dynamic::from("text"));
                element.insert("text".into(), Dynamic::from(val.to_string()));
                element
            });
            $engine.register_fn("text", |_target: rhai::Map, val: $t, class: String| -> rhai::Map {
                let mut element = rhai::Map::new();
                element.insert("type".into(), Dynamic::from("text"));
                element.insert("text".into(), Dynamic::from(val.to_string()));
                element.insert("class".into(), Dynamic::from(class));
                element
            });
        )*
    };
}

fn register_rhai_text_and_progress(engine: &mut Engine) {
    register_text_elements!(engine, String, i64, f64);

    engine.register_fn("progress", |_target: rhai::Map, props: rhai::Map| {
        with_type(props, "progress")
    });
    register_rhai_float_progress(engine);
    register_rhai_integer_progress(engine);
}

/// Registers `ui.progress(value)`, `(value, class)` and `(value, orientation, class)` for a
/// float `value`.
fn register_rhai_float_progress(engine: &mut Engine) {
    engine.register_fn("progress", |_target: rhai::Map, val: f64| {
        progress_element(val, None, None)
    });
    engine.register_fn("progress", |_target: rhai::Map, val: f64, class: String| {
        progress_element(val, None, Some(class))
    });
    engine.register_fn(
        "progress",
        |_target: rhai::Map, val: f64, orientation: String, class: String| {
            progress_element(val, Some(orientation), Some(class))
        },
    );
}

/// Registers `ui.progress(value)`, `(value, class)` and `(value, orientation, class)` for an
/// integer `value`, which fills nothing outside `i32`.
fn register_rhai_integer_progress(engine: &mut Engine) {
    engine.register_fn("progress", |_target: rhai::Map, val: i64| {
        progress_element(integer_progress_value(val), None, None)
    });
    engine.register_fn("progress", |_target: rhai::Map, val: i64, class: String| {
        progress_element(integer_progress_value(val), None, Some(class))
    });
    engine.register_fn(
        "progress",
        |_target: rhai::Map, val: i64, orientation: String, class: String| {
            progress_element(integer_progress_value(val), Some(orientation), Some(class))
        },
    );
}

/// An integer progress value as the float a progress element holds: itself within `i32`, else
/// zero.
fn integer_progress_value(val: i64) -> f64 {
    i32::try_from(val).map_or(0.0, f64::from)
}

/// The progress element filled to `value`, with `orientation` and `class` when given.
fn progress_element(value: f64, orientation: Option<String>, class: Option<String>) -> rhai::Map {
    let mut element = rhai::Map::new();
    element.insert("type".into(), Dynamic::from("progress"));
    element.insert("value".into(), Dynamic::from(value));
    if let Some(orientation) = orientation {
        element.insert("orientation".into(), Dynamic::from(orientation));
    }
    if let Some(class) = class {
        element.insert("class".into(), Dynamic::from(class));
    }
    element
}

fn register_rhai_rect_and_image(engine: &mut Engine) {
    engine.register_fn("rect", |_target: rhai::Map, props: rhai::Map| {
        with_type(props, "rect")
    });
    engine.register_fn("rect", |_target: rhai::Map, class: String| {
        let mut element = rhai::Map::new();
        element.insert("class".into(), Dynamic::from(class));
        with_type(element, "rect")
    });
    engine.register_fn("rect", |_target: rhai::Map| {
        with_type(rhai::Map::new(), "rect")
    });

    engine.register_fn("image", |_target: rhai::Map, props: rhai::Map| {
        with_type(props, "image")
    });
}

pub(super) fn register_rhai_leaf_elements(engine: &mut Engine) {
    register_rhai_text_and_progress(engine);
    register_rhai_rect_and_image(engine);
}
