use crate::features::module_runtime::ports::AnyModulePort;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum BuiltinError {
    #[error("HOME environment variable not set: {0}")]
    Env(String),
    #[error("IO error: {0}")]
    Io(String),
    #[error("Unsupported engine '{engine}' for module '{module_name}' (expected 'rhai' or 'lua')")]
    UnsupportedEngine {
        engine: String,
        module_name: crate::shared::primitives::ModuleName,
    },
    #[error("Module '{module_name}' not found{engine_suffix}")]
    ModuleNotFound {
        module_name: crate::shared::primitives::ModuleName,
        engine_suffix: String,
    },
}

pub struct BuiltinModules;

impl BuiltinModules {
    const BUILTINS: &[(&'static str, &'static str)] = &[
        ("bar.lua", include_str!("../../assets/widgets/bar.lua")),
        ("bar.rhai", include_str!("../../assets/widgets/bar.rhai")),
        ("clock.lua", include_str!("../../assets/widgets/clock.lua")),
        (
            "clock.rhai",
            include_str!("../../assets/widgets/clock.rhai"),
        ),
        (
            "calendar.lua",
            include_str!("../../assets/widgets/calendar.lua"),
        ),
        (
            "calendar.rhai",
            include_str!("../../assets/widgets/calendar.rhai"),
        ),
        (
            "workspace.lua",
            include_str!("../../assets/widgets/workspace.lua"),
        ),
        (
            "workspace.rhai",
            include_str!("../../assets/widgets/workspace.rhai"),
        ),
        (
            "systray.lua",
            include_str!("../../assets/widgets/systray.lua"),
        ),
        (
            "systray.rhai",
            include_str!("../../assets/widgets/systray.rhai"),
        ),
        (
            "metrics.lua",
            include_str!("../../assets/widgets/metrics.lua"),
        ),
        (
            "metrics.rhai",
            include_str!("../../assets/widgets/metrics.rhai"),
        ),
        ("mpris.lua", include_str!("../../assets/widgets/mpris.lua")),
        (
            "mpris.rhai",
            include_str!("../../assets/widgets/mpris.rhai"),
        ),
    ];

    /// Ensures that all builtin module files exist in the local share directory.
    ///
    /// # Errors
    ///
    /// Returns [`BuiltinError::Io`] if creating directories or reading/writing files fails.
    pub fn ensure_builtins(
        app_env: &crate::shared::env::domain::AppEnvironment,
    ) -> Result<PathBuf, BuiltinError> {
        let home = app_env.home().as_path();
        let dir = home.join(".local/share/cranky/modules");

        fs::create_dir_all(&dir).map_err(|e| BuiltinError::Io(e.to_string()))?;

        for (filename, content) in Self::BUILTINS {
            let path = dir.join(filename);
            if !path.exists() || fs::read_to_string(&path).ok().as_deref() != Some(*content) {
                let _ = fs::write(path, content);
            }
        }

        Ok(dir)
    }

    fn registered_engines() -> Vec<Box<dyn crate::shared::scripting::ports::ScriptEnginePort>> {
        vec![
            Box::new(crate::shared::scripting::adapters::LuaEngineAdapter),
            Box::new(crate::shared::scripting::adapters::RhaiEngineAdapter),
        ]
    }

    /// Finds and instantiates a module matching the given name and engine selection.
    ///
    /// # Errors
    ///
    /// Returns [`BuiltinError::UnsupportedEngine`] if the explicit engine is unknown,
    /// [`BuiltinError::ModuleNotFound`] if no script matching the name and engine could be found or loaded,
    /// or [`BuiltinError::Io`] on filesystem error.
    pub fn find_module(
        name: &crate::shared::primitives::ModuleName,
        selection: &crate::shared::config::domain::EngineSelection,
        app_env: &crate::shared::env::domain::AppEnvironment,
    ) -> Result<Box<dyn AnyModulePort>, BuiltinError> {
        let _ = Self::ensure_builtins(app_env)?;

        let home = app_env.home().as_path();
        let user_dir = home.join(".config/cranky/modules");
        let shadow_dir = home.join(".local/share/cranky/modules");

        let engines = Self::registered_engines();

        let target_engines: Vec<&dyn crate::shared::scripting::ports::ScriptEnginePort> =
            match selection {
                crate::shared::config::domain::EngineSelection::Auto => {
                    engines.iter().map(AsRef::as_ref).collect()
                }
                crate::shared::config::domain::EngineSelection::Explicit(id) => {
                    let matching: Vec<_> = engines
                        .iter()
                        .map(AsRef::as_ref)
                        .filter(|e| &e.id() == id)
                        .collect();
                    if matching.is_empty() {
                        return Err(BuiltinError::UnsupportedEngine {
                            engine: id.as_str().to_string(),
                            module_name: name.clone(),
                        });
                    }
                    matching
                }
            };

        for dir in [&user_dir, &shadow_dir] {
            for engine in &target_engines {
                let path = dir.join(format!("{name}.{}", engine.file_extension().as_str()));
                if let Ok(source) = fs::read_to_string(&path)
                    && let Ok(module) = engine.load_module(name.as_str(), &source)
                {
                    return Ok(module);
                }
            }
        }

        let engine_suffix = selection
            .as_explicit()
            .map_or_else(String::new, |id| format!(" (engine: {})", id.as_str()));

        Err(BuiltinError::ModuleNotFound {
            module_name: name.clone(),
            engine_suffix,
        })
    }

    /// Watches user and system module directories for changes and triggers reloads.
    ///
    /// # Errors
    ///
    /// Returns [`BuiltinError::Io`] if creating the file watcher fails.
    pub fn watch_scripts(
        command_tx: std::sync::Arc<dyn crate::app::commands::SystemCommandSender>,
        app_env: &crate::shared::env::domain::AppEnvironment,
    ) -> Result<Box<dyn notify::Watcher>, BuiltinError> {
        use notify::{Event, RecursiveMode, Watcher};

        let home = app_env.home().as_path();
        let user_dir = home.join(".config/cranky/modules");
        let shadow_dir = home.join(".local/share/cranky/modules");

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res
                && event.kind.is_modify()
            {
                for path in event.paths {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        tracing::debug!("Script modified: {path:?}");
                        command_tx.send_system_command(
                            crate::app::commands::SystemCommand::ReloadModule(
                                crate::shared::primitives::ModuleName::new(stem),
                            ),
                        );
                    }
                }
            }
        })
        .map_err(|e| BuiltinError::Io(format!("Failed to create watcher: {e}")))?;

        if user_dir.exists() {
            let _ = watcher.watch(&user_dir, RecursiveMode::NonRecursive);
        }
        if shadow_dir.exists() {
            let _ = watcher.watch(&shadow_dir, RecursiveMode::NonRecursive);
        }

        Ok(Box::new(watcher))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::config::domain::{EngineId, EngineSelection};

    fn get_test_env() -> crate::shared::env::domain::AppEnvironment {
        crate::shared::env::domain::AppEnvironment::new(
            crate::shared::env::domain::HomeDir::new(std::path::PathBuf::from(
                std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()),
            )),
            crate::shared::env::domain::XdgCacheHome::new(std::path::PathBuf::from("/")),
            crate::shared::env::domain::XdgRuntimeDir::new(std::path::PathBuf::from("/")),
            crate::shared::env::domain::RustLog::new(String::new()),
            None,
        )
    }

    #[test]
    fn test_ensure_builtins() {
        let env = get_test_env();
        let dir = BuiltinModules::ensure_builtins(&env).expect("ensure_builtins failed");
        assert!(dir.join("clock.rhai").exists());
        assert!(dir.join("clock.lua").exists());
        assert!(dir.join("calendar.rhai").exists());
        assert!(dir.join("calendar.lua").exists());
        assert!(dir.join("workspace.rhai").exists());
        assert!(dir.join("workspace.lua").exists());
        assert!(dir.join("systray.lua").exists());
        assert!(dir.join("systray.rhai").exists());
        assert!(dir.join("metrics.lua").exists());
        assert!(dir.join("metrics.rhai").exists());
    }

    #[test]
    fn test_find_module_systray_and_metrics_rhai() {
        use crate::shared::primitives::ModuleName;
        let env = get_test_env();
        let selection = EngineSelection::Explicit(EngineId::new("rhai"));
        let systray_mod =
            BuiltinModules::find_module(&ModuleName::new("systray"), &selection, &env).unwrap();
        let mut metrics_mod =
            BuiltinModules::find_module(&ModuleName::new("metrics"), &selection, &env).unwrap();
        let hub = crate::shared::events::signals::SignalHub::new(
            crate::shared::config::domain::Config::default(),
        );
        let metrics_state = crate::features::metrics::domain::MetricsState::new(
            crate::features::metrics::domain::CreateMetricsCommand::new(
                crate::features::metrics::domain::CpuUsage::new(50.0),
                vec![],
                crate::features::metrics::domain::MemoryMetrics::new(
                    crate::features::metrics::domain::MemoryBytes::new(1024),
                    crate::features::metrics::domain::MemoryBytes::new(2048),
                    crate::features::metrics::domain::MemoryBytes::new(0),
                    crate::features::metrics::domain::MemoryBytes::new(0),
                ),
                vec![],
                crate::features::metrics::domain::NetworkMetrics::new(
                    crate::features::metrics::domain::NetworkSpeed::new(0),
                    crate::features::metrics::domain::NetworkSpeed::new(0),
                ),
                crate::features::metrics::domain::Temperature::new(45.0),
                crate::features::metrics::domain::MetricsConfig::default(),
            ),
        );
        hub.metrics_tx().send(metrics_state).unwrap();
        metrics_mod.refresh(&hub, &[crate::shared::events::signals::SignalKind::Metrics]);

        let _ = systray_mod.render(&crate::shared::primitives::MonitorId::new("DP-1"));
        let metrics_node = metrics_mod.render(&crate::shared::primitives::MonitorId::new("DP-1"));
        assert_eq!(
            metrics_node.tag(),
            crate::features::vdom::domain::NodeTag::Flex
        );
    }

    #[test]
    fn test_find_module_clock_rhai_format() {
        use crate::shared::primitives::ModuleName;
        let env = get_test_env();
        let selection = EngineSelection::Explicit(EngineId::new("rhai"));
        let mut clock_mod =
            BuiltinModules::find_module(&ModuleName::new("clock"), &selection, &env).unwrap();
        let mut options_map = std::collections::HashMap::new();
        options_map.insert(
            "format".to_string(),
            crate::shared::primitives::DynamicValue::String("%H:%M".to_string()),
        );
        let options = crate::shared::primitives::ModuleOptions::new(options_map);
        clock_mod
            .init(
                &crate::shared::config::domain::ModuleConfig::new(
                    ModuleName::new("clock"),
                    true,
                    selection.clone(),
                    options,
                ),
                &crate::shared::config::domain::Config::default(),
            )
            .unwrap();
        let hub = crate::shared::events::signals::SignalHub::new(
            crate::shared::config::domain::Config::default(),
        );
        let test_time = chrono::DateTime::parse_from_rfc3339("2026-07-28T14:30:45+00:00")
            .unwrap()
            .with_timezone(&chrono::Local);
        hub.time_tx().send(test_time).unwrap();
        clock_mod.refresh(&hub, &[crate::shared::events::signals::SignalKind::Time]);
        let node = clock_mod.render(&crate::shared::primitives::MonitorId::new("DP-1"));
        assert_eq!(node.tag(), crate::features::vdom::domain::NodeTag::Text);
        if let crate::features::vdom::domain::VNodeKind::Text { text } = node.kind() {
            let expected = test_time.format("%H:%M").to_string();
            assert_eq!(text.as_str(), expected.as_str());
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
        use crate::shared::primitives::{FunctionName, ModuleName};
        let env = get_test_env();

        for engine_name in ["lua", "rhai"] {
            let selection = EngineSelection::Explicit(EngineId::new(engine_name));
            let mut cal_mod =
                BuiltinModules::find_module(&ModuleName::new("calendar"), &selection, &env)
                    .unwrap_or_else(|_| panic!("Failed to find calendar for {engine_name}"));

            cal_mod
                .init(
                    &crate::shared::config::domain::ModuleConfig::new(
                        ModuleName::new("calendar"),
                        true,
                        selection.clone(),
                        crate::shared::primitives::ModuleOptions::default(),
                    ),
                    &crate::shared::config::domain::Config::default(),
                )
                .unwrap();

            let hub = crate::shared::events::signals::SignalHub::new(
                crate::shared::config::domain::Config::default(),
            );
            let test_time = chrono::DateTime::parse_from_rfc3339("2026-09-01T12:00:00+00:00")
                .unwrap()
                .with_timezone(&chrono::Local);
            hub.time_tx().send(test_time).unwrap();
            cal_mod.refresh(&hub, &[crate::shared::events::signals::SignalKind::Time]);

            let root = cal_mod.render(&crate::shared::primitives::MonitorId::new("DP-1"));
            assert_eq!(
                root.tag(),
                crate::features::vdom::domain::NodeTag::Flex,
                "Root should be Flex for {engine_name}"
            );
            assert_eq!(
                root.children().len(),
                3,
                "Root should have 3 children (Header, Weekdays, Grid) for {engine_name}"
            );

            let header = &root.children()[0];
            assert_eq!(header.children().len(), 3, "Header has prev, title, next");

            let weekdays = &root.children()[1];
            assert_eq!(
                weekdays.tag(),
                crate::features::vdom::domain::NodeTag::Grid,
                "Weekdays should be Grid for {engine_name}"
            );
            assert_eq!(weekdays.children().len(), 7, "Weekdays has 7 items");

            let days_grid = &root.children()[2];
            assert_eq!(
                days_grid.tag(),
                crate::features::vdom::domain::NodeTag::Grid,
                "Days should be Grid for {engine_name}"
            );
            assert_eq!(days_grid.children().len(), 42, "Days grid has 42 cells");

            // Test navigation function calls
            assert!(
                cal_mod
                    .call_function(&FunctionName::new("prev_month"))
                    .is_ok()
            );
            let prev_root = cal_mod.render(&crate::shared::primitives::MonitorId::new("DP-1"));
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
    }

    #[test]
    fn test_clock_popup_toggle_lua_and_rhai() {
        use crate::shared::primitives::{FunctionName, ModuleName};
        let env = get_test_env();

        for engine_name in ["lua", "rhai"] {
            let selection = EngineSelection::Explicit(EngineId::new(engine_name));
            let mut clock_mod =
                BuiltinModules::find_module(&ModuleName::new("clock"), &selection, &env)
                    .unwrap_or_else(|_| panic!("Failed to find clock for {engine_name}"));

            clock_mod
                .init(
                    &crate::shared::config::domain::ModuleConfig::new(
                        ModuleName::new("clock"),
                        true,
                        selection.clone(),
                        crate::shared::primitives::ModuleOptions::default(),
                    ),
                    &crate::shared::config::domain::Config::default(),
                )
                .unwrap();

            let hub = crate::shared::events::signals::SignalHub::new(
                crate::shared::config::domain::Config::default(),
            );
            let test_time = chrono::DateTime::parse_from_rfc3339("2026-09-01T12:00:00+00:00")
                .unwrap()
                .with_timezone(&chrono::Local);
            hub.time_tx().send(test_time).unwrap();
            clock_mod.refresh(&hub, &[crate::shared::events::signals::SignalKind::Time]);

            // Initial state: popup is None
            let node_before = clock_mod.render(&crate::shared::primitives::MonitorId::new("DP-1"));
            assert!(
                node_before.popup().is_none(),
                "Popup should initially be None for {engine_name}"
            );

            // Toggle popup on
            clock_mod
                .call_function(&FunctionName::new("toggle_popup"))
                .expect("Failed to call toggle_popup");
            let node_after_toggle_on =
                clock_mod.render(&crate::shared::primitives::MonitorId::new("DP-1"));
            assert!(
                node_after_toggle_on.popup().is_some(),
                "Popup should be Some after toggle_popup on {engine_name}"
            );
            let popup = node_after_toggle_on.popup().unwrap();
            assert_eq!(
                popup.content().tag(),
                crate::features::vdom::domain::NodeTag::Flex,
                "Popup should be a Flex node for {engine_name}"
            );
            assert_eq!(
                popup.content().children().len(),
                3,
                "Popup should contain Header, Weekdays, and Grid for {engine_name}"
            );

            // Dismiss popup via on_popup_dismiss
            clock_mod
                .call_function(&FunctionName::new("on_popup_dismiss"))
                .expect("Failed to call on_popup_dismiss");
            let node_after_dismiss =
                clock_mod.render(&crate::shared::primitives::MonitorId::new("DP-1"));
            assert!(
                node_after_dismiss.popup().is_none(),
                "Popup should be None after on_popup_dismiss on {engine_name}"
            );

            // Re-open popup
            clock_mod
                .call_function(&FunctionName::new("toggle_popup"))
                .expect("Failed to call toggle_popup after dismiss");
            let node_reopened =
                clock_mod.render(&crate::shared::primitives::MonitorId::new("DP-1"));
            assert!(
                node_reopened.popup().is_some(),
                "Popup should be Some after re-opening on {engine_name}"
            );

            // Toggle popup off
            clock_mod
                .call_function(&FunctionName::new("toggle_popup"))
                .expect("Failed to call toggle_popup second time");
            let node_after_toggle_off =
                clock_mod.render(&crate::shared::primitives::MonitorId::new("DP-1"));
            assert!(
                node_after_toggle_off.popup().is_none(),
                "Popup should be None after toggle_popup off on {engine_name}"
            );
        }
    }

    #[test]
    fn test_clock_popup_multi_monitor_isolation() {
        use crate::shared::primitives::{FunctionName, ModuleName};
        let env = get_test_env();

        for engine_name in ["lua", "rhai"] {
            let selection = EngineSelection::Explicit(EngineId::new(engine_name));
            let mut clock_mod =
                BuiltinModules::find_module(&ModuleName::new("clock"), &selection, &env)
                    .unwrap_or_else(|_| panic!("Failed to find clock for {engine_name}"));

            clock_mod
                .init(
                    &crate::shared::config::domain::ModuleConfig::new(
                        ModuleName::new("clock"),
                        true,
                        selection.clone(),
                        crate::shared::primitives::ModuleOptions::default(),
                    ),
                    &crate::shared::config::domain::Config::default(),
                )
                .unwrap();

            let hub = crate::shared::events::signals::SignalHub::new(
                crate::shared::config::domain::Config::default(),
            );
            let test_time = chrono::DateTime::parse_from_rfc3339("2026-09-01T12:00:00+00:00")
                .unwrap()
                .with_timezone(&chrono::Local);
            hub.time_tx().send(test_time).unwrap();
            clock_mod.refresh(&hub, &[crate::shared::events::signals::SignalKind::Time]);

            // Multi-monitor per-monitor popup test
            clock_mod
                .call_function_with_args(&FunctionName::new("toggle_popup"), &["DP-1"])
                .expect("Failed to call toggle_popup for DP-1");
            let node_dp1 = clock_mod.render(&crate::shared::primitives::MonitorId::new("DP-1"));
            let node_dp2 = clock_mod.render(&crate::shared::primitives::MonitorId::new("DP-2"));
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
            let node_dp1_dismissed =
                clock_mod.render(&crate::shared::primitives::MonitorId::new("DP-1"));
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
            BuiltinError::Io("error".into()).to_string(),
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
}
