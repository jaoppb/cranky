//! Builds Rhai modules from script sources and reads back what they render.
use crate::features::module_runtime::ports::{AnyModulePort, ModuleInitError};
use crate::features::vdom::domain::{VNode, VNodeKind};
use crate::shared::config::domain::{Config, ModuleConfig};
use crate::shared::primitives::{FunctionName, ModuleOptions, MonitorId};
use crate::shared::scripting::adapters::rhai::RhaiModule;

/// A Rhai module compiled from `source` and initialised with the default configuration.
pub(super) fn init_module(name: &str, source: &str) -> RhaiModule {
    let mut module = RhaiModule::new(name.into(), source).unwrap();
    let mod_config = ModuleConfig::new(name.into(), ModuleOptions::default());
    module.init(&mod_config, &Config::default()).unwrap();
    module
}

/// The node a Rhai module whose `render` body is `body` returns on DP-1.
pub(super) fn render_body(body: &str) -> VNode {
    let source = format!("fn render(monitor) {{ {body} }}");
    init_module("rendered", &source).render(&MonitorId::new("DP-1"))
}

/// The text of the text node `node`.
pub(super) fn text_of(node: &VNode) -> String {
    let VNodeKind::Text { text } = node.kind() else {
        panic!("expected a text node, got {:?}", node.kind());
    };
    text.as_str().to_string()
}

/// The text `module` renders on `monitor`.
pub(super) fn rendered_text(module: &RhaiModule, monitor: &str) -> String {
    text_of(&module.render(&MonitorId::new(monitor)))
}

/// The class names of `node`, empty when it has none.
pub(super) fn class_names(node: &VNode) -> Vec<String> {
    node.class_names().map_or_else(Vec::new, |list| {
        list.iter().map(|name| name.as_str().to_string()).collect()
    })
}

/// The script error calling `function` of `module` reports.
pub(super) fn call_error(module: &mut RhaiModule, function: &str, args: &[&str]) -> String {
    match module.call_function_with_args(&FunctionName::new(function), args) {
        Err(ModuleInitError::ScriptError(message)) => message,
        other => panic!("expected a script error, got {other:?}"),
    }
}
