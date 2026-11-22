//! What a Rhai module's `refresh` publishes: the time and metrics signals, and the `signals`, `sys`
//! and `cranky` maps it updates without dropping what the script added.
use super::support::{init_module, rendered_text};
use crate::features::module_runtime::ports::AnyModulePort;
use crate::shared::config::domain::Config;
use crate::shared::events::signals::{SignalHub, SignalKind};

#[test]
fn test_rhai_refresh_publishes_time_and_metrics_only_when_changed() {
    let mut module = init_module(
        "signals",
        r"fn render(monitor) { ui.text(`${type_of(signals.time)}|${type_of(signals.metrics)}|${type_of(signals.mpris)}`) }",
    );
    let hub = SignalHub::new(Config::default());
    module.refresh(&hub, &[SignalKind::Mpris]);
    assert_eq!(rendered_text(&module, "DP-1"), "()|()|map");
    module.refresh(&hub, &[SignalKind::Time, SignalKind::Metrics]);
    assert_eq!(rendered_text(&module, "DP-1"), "string|map|map");
}

#[test]
fn test_rhai_refresh_keeps_what_the_script_added_to_signals_and_sys() {
    let mut module = init_module(
        "kept",
        r#"signals.custom = "kept"; sys.monitors.extra = 5;
           fn render(monitor) { ui.text(`${signals.custom}|${sys.monitors.extra}|${cranky.sys.monitors.extra}|${cranky.signals.custom}|${type_of(cranky.signals.time)}`) }"#,
    );
    module.refresh(&SignalHub::new(Config::default()), &[SignalKind::Time]);
    assert_eq!(rendered_text(&module, "DP-1"), "kept|5|5|kept|string");
}

#[test]
fn test_rhai_refresh_without_a_cranky_map_still_publishes_and_refreshes() {
    let mut module = init_module(
        "no_cranky",
        r#"cranky = 0; let refreshed = "no";
           fn refresh() { refreshed = "yes"; }
           fn render(monitor) { ui.text(`${type_of(cranky)}|${type_of(signals.time)}|${refreshed}`) }"#,
    );
    module.refresh(&SignalHub::new(Config::default()), &[SignalKind::Time]);
    assert_eq!(rendered_text(&module, "DP-1"), "i64|string|yes");
}

#[test]
fn test_rhai_refresh_sees_the_signals_and_sys_mirrored_into_cranky() {
    let mut module = init_module(
        "mirrored",
        r#"let seen = "none";
           fn refresh() { seen = `${type_of(cranky.signals.time)}|${type_of(cranky.sys.monitors._list)}`; }
           fn render(monitor) { ui.text(seen) }"#,
    );
    module.refresh(&SignalHub::new(Config::default()), &[SignalKind::Time]);
    assert_eq!(rendered_text(&module, "DP-1"), "string|array");
}
