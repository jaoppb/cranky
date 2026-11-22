use crate::features::module_runtime::ports::AnyModulePort;
use crate::shared::events::signals::{SignalHub, SignalKind};
use crate::shared::primitives::{FunctionName, MonitorId};
use crate::shared::scripting::adapters::rhai::RhaiModule;

#[test]
fn test_rhai_module_lifecycle() {
    let source = "
        fn subscriptions() { return [\"time\", \"hyprland\", \"metrics\"]; }
        fn refresh() {}
        fn render(monitor) {
            return ui.flex([]);
        }
    ";
    let mut module = RhaiModule::new("test_life".into(), source).unwrap();
    let mod_config = crate::shared::config::domain::ModuleConfig::new(
        "test_life".into(),
        crate::shared::primitives::ModuleOptions::default(),
    );
    let config = crate::shared::config::domain::Config::default();
    assert!(module.init(&mod_config, &config).is_ok());

    let subs = module.subscriptions();
    assert!(subs.contains(&SignalKind::Time));
    assert!(subs.contains(&SignalKind::Hyprland));
    assert!(subs.contains(&SignalKind::Metrics));

    let hub = SignalHub::new(crate::shared::config::domain::Config::default());
    module.refresh(&hub, &[SignalKind::Time]);

    let render_node = module.render(&MonitorId::new("DP-1"));
    assert_eq!(
        render_node.tag(),
        crate::features::vdom::domain::NodeTag::Flex
    );
}

#[test]
fn test_rhai_call_function_optional_not_found() {
    let source = r#"
        fn init() {}
        fn refresh() {}
        fn render(monitor) {
            return ui.text("hi");
        }
    "#;
    let mut module = RhaiModule::new("test_optional".into(), source).unwrap();
    let res = module.call_function(&FunctionName::new("non_existent_fn"));
    assert!(
        res.is_ok(),
        "Calling undefined function should return Ok(())"
    );
}

/// The name and source of every built-in Rhai widget.
const BUILTINS: [(&str, &str); 7] = [
    (
        "clock",
        include_str!("../../../../../../assets/widgets/clock.rhai"),
    ),
    (
        "calendar",
        include_str!("../../../../../../assets/widgets/calendar.rhai"),
    ),
    (
        "workspace",
        include_str!("../../../../../../assets/widgets/workspace.rhai"),
    ),
    (
        "systray",
        include_str!("../../../../../../assets/widgets/systray.rhai"),
    ),
    (
        "metrics",
        include_str!("../../../../../../assets/widgets/metrics.rhai"),
    ),
    (
        "mpris",
        include_str!("../../../../../../assets/widgets/mpris.rhai"),
    ),
    (
        "bar",
        include_str!("../../../../../../assets/widgets/bar.rhai"),
    ),
];

#[test]
fn test_rhai_all_builtins() {
    for (name, source) in BUILTINS {
        let res = RhaiModule::new(name.into(), source);
        if let Err(error) = &res {
            eprintln!("ERROR for {name}: {error:?}");
        }
        assert!(
            res.is_ok(),
            "Failed to load builtin rhai module {name}: {:?}",
            res.err()
        );
    }
}

#[test]
fn test_time_signal_is_whole_second_rfc_3339_with_a_numeric_offset() {
    let source = "
        fn subscriptions() { return [\"time\"]; }
        fn refresh() {}
        fn render(monitor) {
            return ui.text(signals.time);
        }
    ";
    let mut module = RhaiModule::new("time_text".into(), source).unwrap();
    let mod_config = crate::shared::config::domain::ModuleConfig::new(
        "time_text".into(),
        crate::shared::primitives::ModuleOptions::default(),
    );
    let config = crate::shared::config::domain::Config::default();
    assert!(module.init(&mod_config, &config).is_ok());

    let hub = SignalHub::new(crate::shared::config::domain::Config::default());
    // 2026-11-18T11:06:40Z
    hub.time_tx()
        .send(cranky_time::LocalTime::utc(1_795_000_000))
        .unwrap();
    module.refresh(&hub, &[SignalKind::Time]);

    let node = module.render(&MonitorId::new("DP-1"));
    let crate::features::vdom::domain::VNodeKind::Text { text } = node.kind() else {
        panic!("Expected a text node, got {:?}", node.tag());
    };
    assert_eq!(text.as_str(), "2026-11-18T11:06:40+00:00");
}
