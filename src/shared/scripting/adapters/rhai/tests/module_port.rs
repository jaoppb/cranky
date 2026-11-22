//! A Rhai module through `AnyModulePort`: the monitor its `render` gets, the node a failing
//! `render` falls back to, the functions it calls by name and the `config` it reads.
use super::support::{call_error, init_module, rendered_text};
use crate::features::module_runtime::ports::AnyModulePort;
use crate::features::vdom::domain::NodeTag;
use crate::shared::config::domain::{Config, ModuleConfig};
use crate::shared::primitives::{DynamicValue, FunctionName, ModuleOptions, MonitorId};
use crate::shared::scripting::adapters::rhai::RhaiModule;
use crate::shared::scripting::adapters::test_support::hub_with_monitors;

#[test]
fn test_rhai_render_gets_the_refreshed_monitor_or_one_from_its_id() {
    let mut module = init_module(
        "monitors",
        r"fn render(monitor) { ui.text(`${monitor.id}|${monitor.name}|${monitor.scale}|${monitor.is_focused}|${monitor.active_workspace_id}`) }",
    );
    module.refresh(&hub_with_monitors(), &[]);
    for (monitor, expected) in [
        ("DP-1", "DP-1|DP-1|2.0|true|4"),
        ("DP-2", "DP-2|DP-2|1.5|false|"),
        ("HDMI-A-1", "HDMI-A-1|HDMI-A-1|1.0|false|"),
    ] {
        assert_eq!(rendered_text(&module, monitor), expected);
    }
}

#[test]
fn test_rhai_render_falls_back_to_an_empty_flex() {
    for source in [
        "fn other() {}",
        r#"fn render(monitor) { throw "boom"; }"#,
        "fn render(monitor) { 42 }",
    ] {
        let module = init_module("fallback", source);
        let node = module.render(&MonitorId::new("DP-1"));
        assert_eq!(node.tag(), NodeTag::Flex, "{source}");
        assert!(
            node.children().is_empty() && node.class_names().is_none(),
            "{source}"
        );
    }
}

const CALL_SOURCE: &str = r#"
    let received = "none";
    fn remember(value) { received = value; }
    fn plain() { received = "plain"; }
    fn fail(value) { throw "broken call"; }
    fn fail_plain() { throw "broken plain"; }
    fn render(monitor) { ui.text(received) }
"#;

#[test]
fn test_rhai_call_function_passes_the_first_argument_or_none() {
    let mut module = init_module("calls", CALL_SOURCE);
    let remember = FunctionName::new("remember");
    module
        .call_function_with_args(&remember, &["DP-1", "extra"])
        .unwrap();
    assert_eq!(rendered_text(&module, "DP-1"), "DP-1");
    module.call_function(&FunctionName::new("plain")).unwrap();
    assert_eq!(rendered_text(&module, "DP-1"), "plain");
}

#[test]
fn test_rhai_call_function_with_an_argument_falls_back_to_no_argument() {
    let mut module = init_module("calls", CALL_SOURCE);
    module
        .call_function_with_args(&FunctionName::new("plain"), &["DP-1"])
        .unwrap();
    assert_eq!(rendered_text(&module, "DP-1"), "plain");
    let missing = FunctionName::new("missing");
    assert!(module.call_function_with_args(&missing, &["DP-1"]).is_ok());
    assert!(module.call_function(&missing).is_ok());
}

#[test]
fn test_rhai_call_function_reports_a_failing_function() {
    let mut module = init_module("calls", CALL_SOURCE);
    let with_argument = call_error(&mut module, "fail", &["DP-1"]);
    assert!(with_argument.contains("broken call"), "{with_argument}");
    let without = call_error(&mut module, "fail_plain", &[]);
    assert!(without.contains("broken plain"), "{without}");
    // A function called with fewer arguments than it takes is reported as missing, so ignored.
    assert!(module.call_function(&FunctionName::new("remember")).is_ok());
    assert_eq!(rendered_text(&module, "DP-1"), "none");
}

#[test]
fn test_rhai_init_ignores_a_failing_init() {
    let module = init_module(
        "failing_init",
        r#"let state = "start"; fn init() { state = "init"; throw "boom"; } fn render(monitor) { ui.text(state) }"#,
    );
    assert_eq!(rendered_text(&module, "DP-1"), "init");
}

#[test]
fn test_rhai_config_holds_the_root_settings_and_the_module_options() {
    let mut module = RhaiModule::new(
        "configured".into(),
        r#"fn render(monitor) { ui.text(`${config.root.name}|${config.root.height}|${config["module"].label}|${config.options.label}|${cranky.config["module"].label}`) }"#,
    )
    .unwrap();
    let options = std::collections::HashMap::from([(
        "label".to_string(),
        DynamicValue::String("hi".to_string()),
    )]);
    let full_config = Config::default();
    let config = ModuleConfig::new("configured".into(), ModuleOptions::new(options));
    module.init(&config, &full_config).unwrap();
    let root = full_config.root();
    let expected = format!(
        "{}|{}|hi|hi|hi",
        root.name().as_str(),
        root.height().value()
    );
    assert_eq!(rendered_text(&module, "DP-1"), expected);
}

#[test]
fn test_rhai_monitor_without_a_name_is_named_after_its_id() {
    let mut module = init_module(
        "unnamed",
        r"fn render(monitor) { ui.text(`${monitor.name}|${type_of(monitor.special_workspace_id)}`) }",
    );
    module.cached_monitors = vec![
        crate::shared::primitives::ScriptMonitorInfo::from_id(&MonitorId::new("DP-3"))
            .with_name(String::new()),
    ];
    assert_eq!(rendered_text(&module, "DP-3"), "DP-3|()");
}
