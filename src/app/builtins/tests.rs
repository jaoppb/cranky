//! Tests of the built-in modules: lookup, deployment and reload, with every script behaviour checked
//! against both the Lua and the Rhai builtins.

use super::*;
use crate::features::vdom::domain::{NodeTag, VNode, VNodeKind};
use crate::shared::config::domain::{Config, EngineId, EngineSelection, ModuleConfig};
use crate::shared::events::signals::{SignalHub, SignalKind};
use crate::shared::primitives::{FunctionName, ModuleOptions, MonitorId};
use crate::test_utils::TestHome;

fn get_test_env() -> crate::test_utils::TestHome {
    crate::test_utils::TestHome::new("builtins")
}

/// The built-in `name` that `find_module` finds for the engine `engine`.
fn find_builtin(env: &TestHome, name: &str, engine: &str) -> Box<dyn AnyModulePort> {
    let selection = EngineSelection::Explicit(EngineId::new(engine));
    BuiltinModules::find_module(
        &crate::shared::primitives::ModuleName::new(name),
        &selection,
        env,
    )
    .unwrap_or_else(|_| panic!("Failed to find {name} for {engine}"))
}

/// The built-in `name` for the engine `engine`, initialised with `options`, pinned to that
/// engine, under the default configuration.
fn init_builtin(
    env: &TestHome,
    (name, engine): (&str, &str),
    options: ModuleOptions,
) -> Box<dyn AnyModulePort> {
    let mut module = find_builtin(env, name, engine);
    let config = ModuleConfig::new(crate::shared::primitives::ModuleName::new(name), options)
        .with_engine(EngineSelection::Explicit(EngineId::new(engine)));
    module.init(&config, &Config::default()).unwrap();
    module
}

/// Refreshes the time signal of `module` from a hub whose time is `unix_seconds`, in UTC.
fn refresh_time(module: &mut dyn AnyModulePort, unix_seconds: i64) {
    let hub = SignalHub::new(Config::default());
    hub.time_tx()
        .send(cranky_time::LocalTime::utc(unix_seconds))
        .unwrap();
    module.refresh(&hub, &[SignalKind::Time]);
}

/// What `module` renders on the monitor `monitor`.
fn render_on(module: &dyn AnyModulePort, monitor: &str) -> VNode {
    module.render(&MonitorId::new(monitor))
}

#[test]
fn test_ensure_builtins() {
    let env = get_test_env();
    let dir = BuiltinModules::ensure_builtins(&env).expect("ensure_builtins failed");
    for file in [
        "clock.rhai",
        "clock.lua",
        "calendar.rhai",
        "calendar.lua",
        "workspace.rhai",
        "workspace.lua",
        "systray.lua",
        "systray.rhai",
        "metrics.lua",
        "metrics.rhai",
    ] {
        assert!(dir.join(file).exists(), "{file}");
    }
}

/// A deployed builtin that no longer matches the embedded one is overwritten with it.
#[test]
fn test_ensure_builtins_rewrites_a_stale_file() {
    let env = get_test_env();
    let dir = BuiltinModules::ensure_builtins(&env).expect("ensure_builtins failed");
    let clock = dir.join("clock.lua");
    let embedded = std::fs::read_to_string(&clock).expect("the deployed clock");
    std::fs::write(&clock, "stale").expect("a stale clock");

    BuiltinModules::ensure_builtins(&env).expect("ensure_builtins failed");

    assert_eq!(std::fs::read_to_string(&clock).expect("clock"), embedded);
}

/// Metrics at 50% CPU, 1 KiB of 2 KiB of memory used, no swap, disk or traffic, and 45 °C.
fn sample_metrics_state() -> crate::features::metrics::domain::MetricsState {
    use crate::features::metrics::domain::{
        CpuMetrics, CpuUsage, CreateMetricsCommand, DeviceMetrics, MemoryBytes, MemoryMetrics,
        MemoryUsage, MetricsConfig, MetricsState, NetworkMetrics, NetworkSpeed, SystemMetrics,
        Temperature,
    };
    let memory = MemoryMetrics::new(
        MemoryUsage::new(MemoryBytes::new(1024), MemoryBytes::new(2048)),
        MemoryUsage::new(MemoryBytes::new(0), MemoryBytes::new(0)),
    );
    let system = SystemMetrics::new(CpuMetrics::new(CpuUsage::new(50.0), vec![]), memory);
    let network = NetworkMetrics::new(NetworkSpeed::new(0), NetworkSpeed::new(0));
    let devices = DeviceMetrics::new(vec![], network, Some(Temperature::new(45.0)));
    MetricsState::new(CreateMetricsCommand::new(
        system,
        devices,
        MetricsConfig::default(),
    ))
}

#[test]
fn test_find_module_systray_and_metrics_rhai() {
    let env = get_test_env();
    let systray_mod = find_builtin(&env, "systray", "rhai");
    let mut metrics_mod = find_builtin(&env, "metrics", "rhai");
    let hub = SignalHub::new(Config::default());
    hub.metrics_tx().send(sample_metrics_state()).unwrap();
    metrics_mod.refresh(&hub, &[SignalKind::Metrics]);

    let _ = render_on(systray_mod.as_ref(), "DP-1");
    let metrics_node = render_on(metrics_mod.as_ref(), "DP-1");
    assert_eq!(metrics_node.tag(), NodeTag::Flex);
}

fn collect_texts(node: &crate::features::vdom::domain::VNode, texts: &mut Vec<String>) {
    if let crate::features::vdom::domain::VNodeKind::Text { text } = node.kind() {
        texts.push(text.as_str().to_string());
    }
    for child in node.children() {
        collect_texts(child, texts);
    }
}

/// Renders the built-in metrics widget of `engine` with temperature on in Celsius and
/// `temperature` as the sampled value, and returns every text it shows.
fn render_metrics_texts(
    engine: &str,
    temperature: Option<crate::features::metrics::domain::Temperature>,
) -> Vec<String> {
    use crate::features::metrics::domain::{MetricsConfig, MetricsState};
    let env = get_test_env();
    let mut metrics_mod = init_builtin(&env, ("metrics", engine), ModuleOptions::default());
    let hub = SignalHub::new(Config::default());
    let config: MetricsConfig =
        serde_json::from_value(serde_json::json!({ "temperature": "celsius" })).unwrap();
    let metrics_state = MetricsState {
        temperature,
        config,
        ..MetricsState::default()
    };
    hub.metrics_tx().send(metrics_state).unwrap();
    metrics_mod.refresh(&hub, &[SignalKind::Metrics]);

    let mut texts = Vec::new();
    collect_texts(&render_on(metrics_mod.as_ref(), "DP-1"), &mut texts);
    texts
}

#[test]
fn test_metrics_widget_shows_the_temperature_lua_and_rhai() {
    for engine in ["lua", "rhai"] {
        let texts = render_metrics_texts(
            engine,
            Some(crate::features::metrics::domain::Temperature::new(45.0)),
        );
        assert!(texts.contains(&"TMP".to_string()), "{engine}: {texts:?}");
        assert!(texts.contains(&"45.0°C".to_string()), "{engine}: {texts:?}");
    }
}

#[test]
fn test_metrics_widget_shows_dashes_without_a_temperature_lua_and_rhai() {
    for engine in ["lua", "rhai"] {
        let texts = render_metrics_texts(engine, None);
        assert!(texts.contains(&"TMP".to_string()), "{engine}: {texts:?}");
        assert!(texts.contains(&"--°C".to_string()), "{engine}: {texts:?}");
    }
}

#[test]
fn test_find_module_clock_rhai_format() {
    let env = get_test_env();
    let options = ModuleOptions::new(std::collections::HashMap::from([(
        "format".to_string(),
        crate::shared::primitives::DynamicValue::String("%H:%M".to_string()),
    )]));
    let mut clock_mod = init_builtin(&env, ("clock", "rhai"), options);
    // 2026-07-28T14:30:45Z
    refresh_time(clock_mod.as_mut(), 1_785_249_045);
    let node = render_on(clock_mod.as_ref(), "DP-1");
    assert_eq!(node.tag(), NodeTag::Text);
    if let VNodeKind::Text { text } = node.kind() {
        assert_eq!(text.as_str(), "14:30");
    } else {
        panic!("Expected Text node");
    }
}

#[test]
fn test_find_module_default_prioritizes_lua() {
    use crate::shared::primitives::ModuleName;
    let env = get_test_env();
    let module =
        BuiltinModules::find_module(&ModuleName::new("clock"), &EngineSelection::Auto, &env);
    assert!(module.is_ok());
}

#[test]
fn test_find_module_explicit_rhai() {
    use crate::shared::primitives::ModuleName;
    let env = get_test_env();
    let selection = EngineSelection::Explicit(EngineId::new("rhai"));
    let module = BuiltinModules::find_module(&ModuleName::new("clock"), &selection, &env);
    assert!(module.is_ok());
}

#[test]
fn test_find_module_explicit_lua() {
    use crate::shared::primitives::ModuleName;
    let env = get_test_env();
    let selection = EngineSelection::Explicit(EngineId::new("lua"));
    let module = BuiltinModules::find_module(&ModuleName::new("clock"), &selection, &env);
    assert!(module.is_ok());
}

#[test]
fn test_find_module_calendar_lua_and_rhai() {
    let env = get_test_env();

    for engine_name in ["lua", "rhai"] {
        let mut cal_mod = init_builtin(&env, ("calendar", engine_name), ModuleOptions::default());
        // 2026-09-01T12:00:00Z
        refresh_time(cal_mod.as_mut(), 1_788_264_000);

        assert_calendar_shape(&render_on(cal_mod.as_ref(), "DP-1"), engine_name);
        assert_calendar_navigates(cal_mod.as_mut());
    }
}

/// Checks the calendar `root` rendered with `engine_name`: a flex of a header (prev, title,
/// next), a seven-item weekday grid and a 42-cell day grid.
fn assert_calendar_shape(root: &VNode, engine_name: &str) {
    assert_eq!(
        root.tag(),
        NodeTag::Flex,
        "Root should be Flex for {engine_name}"
    );
    assert_eq!(
        root.children().len(),
        3,
        "Root should have 3 children (Header, Weekdays, Grid) for {engine_name}"
    );

    let header = &root.children()[0];
    assert_eq!(header.children().len(), 3, "Header has prev, title, next");

    assert_calendar_grids(root, engine_name);
}

/// Checks the weekday and day grids of the calendar `root` rendered with `engine_name`.
fn assert_calendar_grids(root: &VNode, engine_name: &str) {
    let weekdays = &root.children()[1];
    assert_eq!(
        weekdays.tag(),
        NodeTag::Grid,
        "Weekdays should be Grid for {engine_name}"
    );
    assert_eq!(weekdays.children().len(), 7, "Weekdays has 7 items");

    let days_grid = &root.children()[2];
    assert_eq!(
        days_grid.tag(),
        NodeTag::Grid,
        "Days should be Grid for {engine_name}"
    );
    assert_eq!(days_grid.children().len(), 42, "Days grid has 42 cells");
}

/// Checks the calendar's navigation functions: previous month (still rendering its three
/// parts), next month and back to today.
fn assert_calendar_navigates(cal_mod: &mut dyn AnyModulePort) {
    // Test navigation function calls
    assert!(
        cal_mod
            .call_function(&FunctionName::new("prev_month"))
            .is_ok()
    );
    let prev_root = render_on(cal_mod, "DP-1");
    assert_eq!(prev_root.children().len(), 3);

    assert!(
        cal_mod
            .call_function(&FunctionName::new("next_month"))
            .is_ok()
    );
    assert!(
        cal_mod
            .call_function(&FunctionName::new("reset_today"))
            .is_ok()
    );
}

/// The built-in clock for `engine_name`, initialised and refreshed to 2026-09-01T12:00:00Z.
fn noon_clock(env: &TestHome, engine_name: &str) -> Box<dyn AnyModulePort> {
    let mut clock_mod = init_builtin(env, ("clock", engine_name), ModuleOptions::default());
    // 2026-09-01T12:00:00Z
    refresh_time(clock_mod.as_mut(), 1_788_264_000);
    clock_mod
}

/// Calls `function` of `module`, failing the test with `failure` when the call errors.
fn call(module: &mut dyn AnyModulePort, function: &str, failure: &str) {
    module
        .call_function(&FunctionName::new(function))
        .expect(failure);
}

#[test]
fn test_clock_popup_toggle_lua_and_rhai() {
    let env = get_test_env();

    for engine_name in ["lua", "rhai"] {
        let mut clock_mod = noon_clock(&env, engine_name);

        // Initial state: popup is None
        let node_before = render_on(clock_mod.as_ref(), "DP-1");
        assert!(
            node_before.popup().is_none(),
            "Popup should initially be None for {engine_name}"
        );

        assert_toggle_opens_the_calendar(clock_mod.as_mut(), engine_name);
        assert_dismiss_reopen_and_close(clock_mod.as_mut(), engine_name);
    }
}

/// Checks that toggling the popup of `clock_mod` opens one embedding the calendar module.
fn assert_toggle_opens_the_calendar(clock_mod: &mut dyn AnyModulePort, engine_name: &str) {
    // Toggle popup on
    call(clock_mod, "toggle_popup", "Failed to call toggle_popup");
    let node_after_toggle_on = render_on(clock_mod, "DP-1");
    assert!(
        node_after_toggle_on.popup().is_some(),
        "Popup should be Some after toggle_popup on {engine_name}"
    );
    let popup = node_after_toggle_on.popup().unwrap();
    assert_eq!(
        popup.content().tag(),
        NodeTag::Module,
        "Popup content should be a Module node for {engine_name}"
    );
    let VNodeKind::Module { name, .. } = popup.content().kind() else {
        panic!("Popup content should be a Module node for {engine_name}");
    };
    assert_eq!(
        name.as_str(),
        "calendar",
        "Popup should embed the calendar module for {engine_name}"
    );
}

/// The calls that close, reopen and close again an open clock popup: the function, the
/// failure message, whether the popup is open afterwards, and that state in words.
const DISMISS_REOPEN_CLOSE: [(&str, &str, bool, &str); 3] = [
    // Dismiss popup via on_popup_dismiss
    (
        "on_popup_dismiss",
        "Failed to call on_popup_dismiss",
        false,
        "None after on_popup_dismiss",
    ),
    // Re-open popup
    (
        "toggle_popup",
        "Failed to call toggle_popup after dismiss",
        true,
        "Some after re-opening",
    ),
    // Toggle popup off
    (
        "toggle_popup",
        "Failed to call toggle_popup second time",
        false,
        "None after toggle_popup off",
    ),
];

/// Checks that the open popup of `clock_mod` closes on dismiss, reopens on toggle and
/// closes on a second toggle.
fn assert_dismiss_reopen_and_close(clock_mod: &mut dyn AnyModulePort, engine_name: &str) {
    for (function, failure, open, state) in DISMISS_REOPEN_CLOSE {
        call(clock_mod, function, failure);
        assert_eq!(
            render_on(clock_mod, "DP-1").popup().is_some(),
            open,
            "Popup should be {state} on {engine_name}"
        );
    }
}

#[test]
fn test_clock_popup_multi_monitor_isolation() {
    let env = get_test_env();

    for engine_name in ["lua", "rhai"] {
        let mut clock_mod = noon_clock(&env, engine_name);

        // Multi-monitor per-monitor popup test
        clock_mod
            .call_function_with_args(&FunctionName::new("toggle_popup"), &["DP-1"])
            .expect("Failed to call toggle_popup for DP-1");
        let node_dp1 = render_on(clock_mod.as_ref(), "DP-1");
        let node_dp2 = render_on(clock_mod.as_ref(), "DP-2");
        assert!(
            node_dp1.popup().is_some(),
            "DP-1 should have popup on {engine_name}"
        );
        assert!(
            node_dp2.popup().is_none(),
            "DP-2 should NOT have popup on {engine_name}"
        );

        // Dismiss DP-1
        clock_mod
            .call_function_with_args(&FunctionName::new("on_popup_dismiss"), &["DP-1"])
            .expect("Failed to call on_popup_dismiss for DP-1");
        let node_dp1_dismissed = render_on(clock_mod.as_ref(), "DP-1");
        assert!(
            node_dp1_dismissed.popup().is_none(),
            "DP-1 popup should be None after dismiss on {engine_name}"
        );
    }
}

#[test]
fn test_find_module_unsupported_engine() {
    use crate::shared::primitives::ModuleName;
    let env = get_test_env();
    let selection = EngineSelection::Explicit(EngineId::new("python"));
    let err = BuiltinModules::find_module(&ModuleName::new("clock"), &selection, &env)
        .err()
        .expect("Expected error");
    assert_eq!(
        err,
        BuiltinError::UnsupportedEngine {
            engine: "python".to_string(),
            module_name: ModuleName::new("clock"),
        }
    );
}

#[test]
fn test_find_module_not_found() {
    use crate::shared::primitives::ModuleName;
    let env = get_test_env();
    let err = BuiltinModules::find_module(
        &ModuleName::new("nonexistent_module_test"),
        &EngineSelection::Auto,
        &env,
    )
    .err()
    .expect("Expected error");
    assert_eq!(
        err,
        BuiltinError::ModuleNotFound {
            module_name: ModuleName::new("nonexistent_module_test"),
            engine_suffix: String::new(),
        }
    );
}

#[test]
fn test_find_module_not_found_with_engine() {
    use crate::shared::primitives::ModuleName;
    let env = get_test_env();
    let selection = EngineSelection::Explicit(EngineId::new("rhai"));
    let err = BuiltinModules::find_module(
        &ModuleName::new("nonexistent_module_test"),
        &selection,
        &env,
    )
    .err()
    .expect("Expected error");
    assert_eq!(
        err,
        BuiltinError::ModuleNotFound {
            module_name: ModuleName::new("nonexistent_module_test"),
            engine_suffix: " (engine: rhai)".to_string(),
        }
    );
}

#[test]
fn test_builtin_error_display() {
    use crate::shared::primitives::ModuleName;
    assert_eq!(
        BuiltinError::Env("var".into()).to_string(),
        "HOME environment variable not set: var"
    );
    assert_eq!(
        BuiltinError::InputOutput("error".into()).to_string(),
        "IO error: error"
    );
    assert_eq!(
        BuiltinError::UnsupportedEngine {
            engine: "py".into(),
            module_name: ModuleName::new("mod"),
        }
        .to_string(),
        "Unsupported engine 'py' for module 'mod' (expected 'rhai' or 'lua')"
    );
    assert_eq!(
        BuiltinError::ModuleNotFound {
            module_name: ModuleName::new("mod"),
            engine_suffix: " (engine: rhai)".to_string(),
        }
        .to_string(),
        "Module 'mod' not found (engine: rhai)"
    );
}

/// Writes `source` as the module script `file` in the user module directory of `home`.
fn write_user_module(home: &crate::test_utils::TestHome, file: &str, source: &str) {
    let dir = home.home().as_path().join(".config/cranky/modules");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(file), source).unwrap();
}

/// A Lua module script that renders `text`.
fn lua_text_module(text: &str) -> String {
    format!("function render(monitor) return ui.text(\"{text}\") end")
}

/// A Rhai module script that renders `text`.
fn rhai_text_module(text: &str) -> String {
    format!("fn render(monitor) {{ ui.text(\"{text}\") }}")
}

/// The text rendered on DP-1 by the module `name` that `find_module` picks for `selection`.
fn found_text(
    home: &crate::test_utils::TestHome,
    name: &str,
    selection: &EngineSelection,
) -> String {
    use crate::shared::primitives::ModuleName;
    let mut module = BuiltinModules::find_module(&ModuleName::new(name), selection, home)
        .unwrap_or_else(|error| panic!("{name}: {error}"));
    module
        .init(
            &crate::shared::config::domain::ModuleConfig::new(
                ModuleName::new(name),
                crate::shared::primitives::ModuleOptions::default(),
            ),
            &crate::shared::config::domain::Config::default(),
        )
        .unwrap();
    let node = module.render(&crate::shared::primitives::MonitorId::new("DP-1"));
    let crate::features::vdom::domain::VNodeKind::Text { text } = node.kind() else {
        panic!("{name}: expected a text node, got {:?}", node.kind());
    };
    text.as_str().to_string()
}

#[test]
fn test_find_module_auto_prefers_lua_over_rhai() {
    let home = get_test_env();
    write_user_module(&home, "pick.lua", &lua_text_module("lua"));
    write_user_module(&home, "pick.rhai", &rhai_text_module("rhai"));

    assert_eq!(found_text(&home, "pick", &EngineSelection::Auto), "lua");
}

#[test]
fn test_find_module_explicit_engine_skips_the_other_engine() {
    let home = get_test_env();
    write_user_module(&home, "pick.lua", &lua_text_module("lua"));
    write_user_module(&home, "pick.rhai", &rhai_text_module("rhai"));

    let rhai = EngineSelection::Explicit(EngineId::new("rhai"));
    assert_eq!(found_text(&home, "pick", &rhai), "rhai");
}

/// The user directory is searched with every engine before the deployed builtins: a user
/// Rhai clock wins over the deployed Lua one even though Lua comes first.
#[test]
fn test_find_module_user_script_of_any_engine_beats_a_deployed_builtin() {
    let home = get_test_env();
    write_user_module(&home, "clock.rhai", &rhai_text_module("user clock"));

    assert_eq!(
        found_text(&home, "clock", &EngineSelection::Auto),
        "user clock"
    );
}

/// A script that fails to load is skipped for the next candidate. Rhai compiles when the
/// module loads (a Lua chunk only runs at `init`, so a broken Lua script is still found).
#[test]
fn test_find_module_falls_through_a_script_that_fails_to_load() {
    let home = get_test_env();
    write_user_module(&home, "pick.rhai", "fn render(monitor) {");
    let shadow = BuiltinModules::ensure_builtins(&home).unwrap();
    fs::write(shadow.join("pick.lua"), lua_text_module("deployed lua")).unwrap();

    assert_eq!(
        found_text(&home, "pick", &EngineSelection::Auto),
        "deployed lua"
    );
}

/// Starts watching the module directories of `home`, returning the watcher and the commands
/// it sends.
fn watch(
    home: &crate::test_utils::TestHome,
) -> (
    Box<dyn notify::Watcher>,
    std::sync::mpsc::Receiver<crate::app::commands::SystemCommand>,
) {
    let (tx, rx) = std::sync::mpsc::channel();
    let sender = move |command: crate::app::commands::SystemCommand| {
        let _ = tx.send(command);
    };
    let watcher = BuiltinModules::watch_scripts(std::sync::Arc::new(sender), home).unwrap();
    (watcher, rx)
}

/// Every module asked to reload until no command arrives for two seconds, in order.
fn reloads(rx: &std::sync::mpsc::Receiver<crate::app::commands::SystemCommand>) -> Vec<String> {
    let mut names = Vec::new();
    while let Ok(command) = rx.recv_timeout(std::time::Duration::from_secs(2)) {
        let crate::app::commands::SystemCommand::ReloadModule(name) = command else {
            panic!("expected a module reload, got {command:?}");
        };
        names.push(name.as_str().to_owned());
    }
    names
}

#[test]
fn test_watching_reloads_a_script_modified_in_either_module_directory() {
    let home = get_test_env();
    let shadow = BuiltinModules::ensure_builtins(&home).unwrap();
    write_user_module(&home, "mine.lua", &lua_text_module("before"));
    let (_watcher, rx) = watch(&home);

    write_user_module(&home, "mine.lua", &lua_text_module("after"));
    fs::write(shadow.join("clock.lua"), lua_text_module("edited")).unwrap();

    let names = reloads(&rx);
    assert!(names.contains(&"mine".to_owned()), "{names:?}");
    assert!(names.contains(&"clock".to_owned()), "{names:?}");
}

#[test]
fn test_watching_ignores_removals_and_creations() {
    let home = get_test_env();
    write_user_module(&home, "gone.lua", &lua_text_module("gone"));
    let user = home.home().as_path().join(".config/cranky/modules");
    let (_watcher, rx) = watch(&home);

    fs::remove_file(user.join("gone.lua")).unwrap();
    // Created but never written: a create event, no modification.
    fs::File::create(user.join("touched.lua")).unwrap();
    write_user_module(&home, "kept.lua", &lua_text_module("kept"));

    let names = reloads(&rx);
    assert!(names.iter().all(|name| name == "kept"), "{names:?}");
    assert_ne!(names, Vec::<String>::new());
}

/// A modified script is logged at debug level under the `cranky::app::builtins` target, then
/// reloaded under its file stem.
#[test]
fn test_a_modified_script_is_logged_under_the_builtins_target() {
    let (tx, rx) = std::sync::mpsc::channel();
    let sender = move |command: crate::app::commands::SystemCommand| {
        let _ = tx.send(command);
    };

    let logs = crate::features::module_runtime::test_support::capture_logs(|| {
        reload_script(Path::new("/modules/clock.lua"), &sender);
    });

    assert_eq!(
        logs,
        "DEBUG cranky::app::builtins: Script modified: \"/modules/clock.lua\"\n"
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        crate::app::commands::SystemCommand::ReloadModule("clock".into())
    );
}

#[test]
fn test_watching_starts_even_when_no_module_directory_exists() {
    let home = get_test_env();
    // `watch` fails the test when `watch_scripts` returns an error.
    let _started = watch(&home);
}

/// A text measurer that gives every character 7px and every line 16px.
struct Glyphs;

impl crate::features::layout_engine::domain::TextMeasurer for Glyphs {
    fn measure(
        &mut self,
        text: &str,
        _font: crate::shared::rendering::ports::text::TextFont<'_>,
    ) -> crate::shared::primitives::geometry::Size {
        crate::shared::primitives::geometry::Size::new(
            u32::try_from(text.chars().count())
                .unwrap_or(0)
                .saturating_mul(7),
            16,
        )
    }
}

/// Resolves styles from the shipped base and calendar sheets.
fn calendar_style_resolver() -> crate::features::styling::adapters::fs_loader::CompositeStyleResolver
{
    use crate::features::styling::adapters::lightningcss::LightningCssAdapter;
    use crate::features::styling::domain::StyleSheetName;
    use crate::features::styling::ports::CssParserPort;

    let parser = LightningCssAdapter::new();
    let sheets = [
        ("base", include_str!("../../../assets/styles/base.css")),
        (
            "calendar",
            include_str!("../../../assets/styles/calendar.css"),
        ),
    ]
    .into_iter()
    .map(|(sheet_name, css)| {
        parser
            .parse_stylesheet(StyleSheetName::new(sheet_name).unwrap(), css)
            .unwrap()
    })
    .collect();
    crate::features::styling::adapters::fs_loader::CompositeStyleResolver::new(sheets)
}

/// The built-in calendar of `engine_name`, refreshed to 2026-10-15T12:00:00Z, styled by
/// `resolver` and laid out in a 280px-wide host.
fn lay_out_calendar(
    env: &TestHome,
    engine_name: &str,
    resolver: &crate::features::styling::adapters::fs_loader::CompositeStyleResolver,
) -> crate::features::layout_engine::domain::RenderNode {
    use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
    use crate::features::layout_engine::domain::LayoutRoot;
    use crate::features::layout_engine::ports::LayoutEnginePort;
    use crate::shared::primitives::SizeConstraint;
    use crate::shared::primitives::geometry::Position;

    let mut cal = init_builtin(env, ("calendar", engine_name), ModuleOptions::default());
    // 2026-10-15T12:00:00Z
    refresh_time(cal.as_mut(), 1_792_065_600);

    let styled = render_on(cal.as_ref(), "DP-1").resolve_styles(resolver, None);
    let mut engine = TaffyLayoutAdapter::new();
    engine
        .calculate_layout(
            LayoutRoot::new(
                styled,
                Position::new(0, 0),
                SizeConstraint::new(Some(280), None),
            ),
            &mut Glyphs,
        )
        .unwrap()
}

/// The x of the first seven day cells, one week, of the laid-out calendar `tree`.
fn first_week_columns(
    tree: &crate::features::layout_engine::domain::RenderNode,
    engine_name: &str,
) -> Vec<i32> {
    use crate::features::layout_engine::domain::RenderNode;
    let RenderNode::Flex { children, .. } = tree else {
        panic!("{engine_name}: flex root")
    };
    let RenderNode::Grid { children: days, .. } = &children[2] else {
        panic!("{engine_name}: day grid")
    };
    days.iter().take(7).map(|day| day.rect().x()).collect()
}

/// Checks that the columns at `column_positions` spread more than 30px apart at an even
/// step.
fn assert_even_columns(column_positions: &[i32], engine_name: &str) {
    let step = column_positions[1]
        .checked_sub(column_positions[0])
        .unwrap();
    assert!(
        step > 30,
        "{engine_name}: columns should spread, got {column_positions:?}"
    );
    assert!(
        column_positions.windows(2).all(|pair| {
            let gap = pair[1].checked_sub(pair[0]).unwrap();
            gap.checked_sub(step).unwrap().abs() <= 1
        }),
        "{engine_name}: even columns {column_positions:?}"
    );
}

/// Issue #9, end to end on the shipped assets: the calendar laid out at
/// the 280px host its embedder resolved, for both script engines.
#[test]
fn test_calendar_fills_its_host_lua_and_rhai() {
    let resolver = calendar_style_resolver();
    let env = get_test_env();
    let mut laid_out = Vec::new();

    for engine_name in ["lua", "rhai"] {
        let tree = lay_out_calendar(&env, engine_name, &resolver);

        // The root fills the 280px host; the 260px min-height is gone,
        // so the popup follows the content.
        assert_eq!(tree.rect().width(), 280, "{engine_name}: root fills host");
        assert!(
            tree.rect().height() < 260,
            "{engine_name}: no dead space, got {}",
            tree.rect().height()
        );

        // 7 equal columns spanning the padded root (280 - 2*12 - 2*1 border).
        let column_positions = first_week_columns(&tree, engine_name);
        assert_even_columns(&column_positions, engine_name);
        laid_out.push((tree.rect().height(), column_positions));
    }
    assert_eq!(
        laid_out[0], laid_out[1],
        "lua and rhai must lay out identically"
    );
}
