//! The `sys` functions a Rhai script calls: environment variables and the monitor lookups.
use super::support::{init_module, render_body, rendered_text, text_of};
use crate::features::module_runtime::ports::AnyModulePort;
use crate::shared::scripting::adapters::test_support::hub_with_monitors;

#[test]
fn test_rhai_sys_env_reads_a_variable_or_unit() {
    let path = text_of(&render_body(r#"ui.text(sys.env("PATH"))"#));
    assert_eq!(Some(path), std::env::var("PATH").ok());
    let missing = render_body(r#"ui.text(type_of(sys.env("CRANKY_TEST_SURELY_UNSET_VARIABLE")))"#);
    assert_eq!(text_of(&missing), "()");
}

const MONITORS_SOURCE: &str = r#"
    fn render(monitor) {
        let ids = sys.monitors.all().map(|entry| entry.id);
        ids.sort();
        ui.text(`${ids}|${sys.monitors.focused().id}|${sys.monitors.by_name("DP-2").id}|${type_of(sys.monitors.by_name("X"))}|${sys.monitors.current(monitor).id}`)
    }
"#;

#[test]
fn test_rhai_sys_monitors_list_find_and_focus() {
    let mut module = init_module("monitors", MONITORS_SOURCE);
    module.refresh(&hub_with_monitors(), &[]);
    assert_eq!(
        rendered_text(&module, "DP-2"),
        r#"["DP-1", "DP-2"]|DP-1|DP-2|()|DP-2"#
    );
}

#[test]
fn test_rhai_sys_monitor_lookups_skip_entries_that_are_no_monitor() {
    let node = render_body(
        r#"let monitors = #{ _list: [1, #{ name: "A" }, #{ name: "B", is_focused: true }] };
           ui.text(`${all(monitors).len()}|${focused(monitors).name}|${by_name(monitors, "B").name}`)"#,
    );
    assert_eq!(text_of(&node), "3|B|B");
}

#[test]
fn test_rhai_sys_monitor_lookups_without_a_list_find_nothing() {
    let node = render_body(
        r#"let monitors = #{ _list: [#{ name: "A", is_focused: false }] };
           ui.text(`${all(#{}).len()}|${type_of(focused(#{}))}|${type_of(focused(monitors))}|${type_of(by_name(#{}, "A"))}`)"#,
    );
    assert_eq!(text_of(&node), "0|()|()|()");
}

#[test]
fn test_rhai_monitor_to_string_is_its_id_or_empty() {
    let node = render_body(r#"ui.text(`${to_string(#{ id: "DP-3" })}|${to_string(#{ id: 4 })}|`)"#);
    assert_eq!(text_of(&node), "DP-3||");
}
