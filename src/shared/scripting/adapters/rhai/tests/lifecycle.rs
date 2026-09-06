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
        true,
        crate::shared::config::domain::EngineSelection::Auto,
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

#[test]
fn test_rhai_all_builtins() {
    for (name, source) in [
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
        ("bar", include_str!("../../../../../../assets/widgets/bar.rhai")),
    ] {
        let res = RhaiModule::new(name.into(), source);
        if let Err(e) = &res {
            eprintln!("ERROR for {name}: {e:?}");
        }
        assert!(
            res.is_ok(),
            "Failed to load builtin rhai module {name}: {:?}",
            res.err()
        );
    }
}
