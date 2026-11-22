//! A Lua module through `AnyModulePort`: its init errors, the monitor its `render` gets, the node
//! a failing `render` falls back to, and the functions it calls by name.
use super::support::text_of;
use crate::features::module_runtime::ports::{AnyModulePort, ModuleInitError};
use crate::features::vdom::domain::{NodeTag, VNode};
use crate::shared::config::domain::{Config, ModuleConfig};
use crate::shared::primitives::{FunctionName, ModuleOptions, MonitorId};
use crate::shared::scripting::adapters::lua::LuaModule;
use crate::shared::scripting::adapters::test_support::hub_with_monitors;

/// A Lua module running `source`, initialised with the default configuration.
fn init_module(name: &str, source: &str) -> Result<LuaModule, ModuleInitError> {
    let mut module = LuaModule::new(name.to_string(), source.to_string());
    let config = ModuleConfig::new(name.into(), ModuleOptions::default());
    module.init(&config, &Config::default())?;
    Ok(module)
}

/// The error message of initialising a Lua module running `source`.
fn init_error(name: &str, source: &str) -> String {
    match init_module(name, source) {
        Err(ModuleInitError::ScriptError(message)) => message,
        other => panic!("expected a script error, got {other:?}"),
    }
}

fn assert_empty_flex(node: &VNode) {
    assert_eq!(node.tag(), NodeTag::Flex);
    assert!(node.children().is_empty() && node.class_names().is_none());
}

#[test]
fn test_lua_init_reports_load_and_init_errors_with_the_module_name() {
    let load = init_error("broken", "this is not lua");
    assert!(load.starts_with("Lua load error in broken: "), "{load}");
    let init = init_error("failing", r#"function init() error("boom") end"#);
    assert!(init.starts_with("Lua init error in failing: "), "{init}");
    assert!(init.contains("boom"), "{init}");
}

#[test]
fn test_lua_init_runs_the_script_then_its_init() {
    let module = init_module(
        "ordered",
        r#"order = "load"
        function init() order = order .. ",init" end
        function render(monitor) return ui.text(order) end"#,
    )
    .unwrap();
    assert_eq!(
        text_of(&module.render(&MonitorId::new("DP-1"))),
        "load,init"
    );
}

const MONITOR_SOURCE: &str = r#"
    function render(monitor)
        return ui.text(monitor.id .. "|" .. monitor.scale .. "|" .. tostring(monitor.is_focused))
    end
"#;

#[test]
fn test_lua_render_gets_the_refreshed_monitor_or_one_from_its_id() {
    let mut module = init_module("monitors", MONITOR_SOURCE).unwrap();
    module.refresh(&hub_with_monitors(), &[]);
    for (monitor, expected) in [
        ("DP-1", "DP-1|2.0|true"),
        ("DP-2", "DP-2|1.5|false"),
        ("HDMI-A-1", "HDMI-A-1|1.0|false"),
    ] {
        assert_eq!(text_of(&module.render(&MonitorId::new(monitor))), expected);
    }
}

#[test]
fn test_lua_render_skips_raw_monitors_that_are_no_monitor() {
    let source = format!(
        r#"{MONITOR_SOURCE} function refresh() table.insert(_raw_monitors, 1, "DP-2") end"#
    );
    let mut module = init_module("skipping", &source).unwrap();
    module.refresh(&hub_with_monitors(), &[]);
    assert_eq!(
        text_of(&module.render(&MonitorId::new("DP-2"))),
        "DP-2|1.5|false"
    );
}

#[test]
fn test_lua_render_falls_back_to_an_empty_flex() {
    for source in [
        "",
        r#"function render(monitor) error("boom") end"#,
        "function render(monitor) return 5 end",
    ] {
        let module = init_module("fallback", source).unwrap();
        assert_empty_flex(&module.render(&MonitorId::new("DP-1")));
    }
}

const CALL_SOURCE: &str = r#"
    received = "none"
    function remember(value) received = tostring(value) end
    function fail() error("broken call") end
    function render(monitor) return ui.text(received) end
"#;

#[test]
fn test_lua_call_function_passes_only_the_first_argument() {
    let mut module = init_module("calls", CALL_SOURCE).unwrap();
    let remember = FunctionName::new("remember");
    module
        .call_function_with_args(&remember, &["DP-1", "extra"])
        .unwrap();
    assert_eq!(text_of(&module.render(&MonitorId::new("DP-1"))), "DP-1");
    module.call_function(&remember).unwrap();
    assert_eq!(text_of(&module.render(&MonitorId::new("DP-1"))), "nil");
}

#[test]
fn test_lua_call_function_ignores_a_missing_function_and_reports_a_failing_one() {
    let mut module = init_module("calls", CALL_SOURCE).unwrap();
    assert!(module.call_function(&FunctionName::new("missing")).is_ok());
    let Err(ModuleInitError::ScriptError(message)) =
        module.call_function_with_args(&FunctionName::new("fail"), &["x"])
    else {
        panic!("expected a script error");
    };
    assert!(
        message.starts_with("Failed to call function 'fail': "),
        "{message}"
    );
    assert!(message.contains("broken call"), "{message}");
}

#[test]
fn test_lua_refresh_survives_a_failed_sync() {
    let source = r#"
        setmetatable(signals, { __newindex = function() error("read only") end })
        ran = false
        function refresh() ran = true end
        function render(monitor) return ui.text(tostring(ran)) end
    "#;
    let mut module = init_module("read_only", source).unwrap();
    let hub = hub_with_monitors();
    module.refresh(&hub, &[crate::shared::events::signals::SignalKind::Time]);
    assert_eq!(text_of(&module.render(&MonitorId::new("DP-1"))), "false");
    module.refresh(&hub, &[crate::shared::events::signals::SignalKind::Mpris]);
    assert_eq!(text_of(&module.render(&MonitorId::new("DP-1"))), "true");
}
