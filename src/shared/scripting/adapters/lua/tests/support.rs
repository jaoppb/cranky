//! Builds Lua states with the module API and reads back the nodes their scripts return.
use crate::features::vdom::domain::{VNode, VNodeKind};
use crate::shared::scripting::adapters::lua::{register_cranky_api, value_to_vnode};
use mlua::Lua;

/// A Lua state with the module API registered.
pub(super) fn api_state() -> Lua {
    let lua = Lua::new();
    register_cranky_api(&lua).expect("API registration failed");
    lua
}

/// The node the Lua chunk `script` returns.
pub(super) fn eval_node(script: &str) -> VNode {
    let lua = api_state();
    let val = lua.load(script).eval::<mlua::Value>().expect("Eval failed");
    value_to_vnode(&lua, val).expect("Conversion failed")
}

/// The error evaluating the Lua chunk `script` and converting its value to a node, as text.
pub(super) fn eval_node_error(script: &str) -> String {
    let lua = api_state();
    lua.load(script)
        .eval::<mlua::Value>()
        .and_then(|val| value_to_vnode(&lua, val))
        .expect_err("expected an error")
        .to_string()
}

/// The class names of `node`, empty when it has none.
pub(super) fn class_names(node: &VNode) -> Vec<String> {
    node.class_names().map_or_else(Vec::new, |list| {
        list.iter().map(|name| name.as_str().to_string()).collect()
    })
}

/// The text of the text node `node`.
pub(super) fn text_of(node: &VNode) -> String {
    let VNodeKind::Text { text } = node.kind() else {
        panic!("expected a text node, got {:?}", node.kind());
    };
    text.as_str().to_string()
}
