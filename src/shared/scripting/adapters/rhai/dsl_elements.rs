use super::dsl_leaf::register_rhai_leaf_elements;
use super::helpers::with_type;
use rhai::{Dynamic, Engine};

macro_rules! register_container_elements {
    ($engine:expr, $($name:literal),*) => {
        $(
            $engine.register_fn($name, |_target: rhai::Map, children: rhai::Array| -> rhai::Map {
                let mut element = rhai::Map::new();
                element.insert("type".into(), Dynamic::from($name));
                element.insert("children".into(), Dynamic::from(children));
                element
            });
            $engine.register_fn($name, |_target: rhai::Map, mut props: rhai::Map| -> rhai::Map {
                props.insert("type".into(), Dynamic::from($name));
                props
            });
            $engine.register_fn($name, |_target: rhai::Map| -> rhai::Map {
                let mut element = rhai::Map::new();
                element.insert("type".into(), Dynamic::from($name));
                element.insert("children".into(), Dynamic::from(rhai::Array::new()));
                element
            });
        )*
    };
}

fn register_rhai_flex_and_grid(engine: &mut Engine) {
    register_container_elements!(engine, "flex", "grid");
}

/// Registers `ui.module` and its aliases, each taking props, a module name, or a name and a
/// class.
fn register_rhai_module_aliases(engine: &mut Engine) {
    for fn_name in ["module", "widget", "load_module", "mod_element"] {
        engine.register_fn(fn_name, |_target: rhai::Map, props: rhai::Map| {
            with_type(props, "module")
        });
        engine.register_fn(fn_name, |_target: rhai::Map, name: String| {
            module_element(name, None)
        });
        engine.register_fn(
            fn_name,
            |_target: rhai::Map, name: String, class: String| module_element(name, Some(class)),
        );
    }
}

/// The element embedding the module `name`, with `class` when given.
fn module_element(name: String, class: Option<String>) -> rhai::Map {
    let mut element = rhai::Map::new();
    element.insert("type".into(), Dynamic::from("module"));
    element.insert("name".into(), Dynamic::from(name));
    if let Some(class) = class {
        element.insert("class".into(), Dynamic::from(class));
    }
    element
}

/// Registers `ui.popup(props)` and `ui.panel(props)`, also callable without `ui`.
fn register_rhai_popup_and_panel(engine: &mut Engine) {
    engine.register_fn("popup", |_target: rhai::Map, props: rhai::Map| {
        with_type(props, "popup")
    });
    engine.register_fn("panel", |_target: rhai::Map, props: rhai::Map| {
        with_type(props, "panel")
    });
    engine.register_fn("popup", |props: rhai::Map| with_type(props, "popup"));
    engine.register_fn("panel", |props: rhai::Map| with_type(props, "panel"));
}

pub(super) fn register_rhai_ui_elements(engine: &mut Engine) {
    register_rhai_flex_and_grid(engine);
    register_rhai_leaf_elements(engine);
    register_rhai_module_aliases(engine);
    register_rhai_popup_and_panel(engine);
}
