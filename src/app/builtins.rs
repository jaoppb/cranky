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
        ("hour.lua", include_str!("../../assets/widgets/hour.lua")),
        ("hour.rhai", include_str!("../../assets/widgets/hour.rhai")),
        (
            "workspace.lua",
            include_str!("../../assets/widgets/workspace.lua"),
        ),
        (
            "workspace.rhai",
            include_str!("../../assets/widgets/workspace.rhai"),
        ),
        (
            "applet.lua",
            include_str!("../../assets/widgets/applet.lua"),
        ),
        (
            "applet.rhai",
            include_str!("../../assets/widgets/applet.rhai"),
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

    pub fn ensure_builtins(
        app_env: &crate::shared::env::domain::AppEnvironment,
    ) -> Result<PathBuf, BuiltinError> {
        let home = app_env.home().as_path();
        let dir = PathBuf::from(home).join(".local/share/cranky/modules");

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

    pub fn find_module(
        name: &crate::shared::primitives::ModuleName,
        selection: &crate::shared::config::domain::EngineSelection,
        app_env: &crate::shared::env::domain::AppEnvironment,
    ) -> Result<Box<dyn AnyModulePort>, BuiltinError> {
        let _ = Self::ensure_builtins(app_env)?;

        let home = app_env.home().as_path();
        let user_dir = PathBuf::from(home).join(".config/cranky/modules");
        let shadow_dir = PathBuf::from(home).join(".local/share/cranky/modules");

        let engines = Self::registered_engines();

        let target_engines: Vec<&dyn crate::shared::scripting::ports::ScriptEnginePort> =
            match selection {
                crate::shared::config::domain::EngineSelection::Auto => {
                    engines.iter().map(|e| e.as_ref()).collect()
                }
                crate::shared::config::domain::EngineSelection::Explicit(id) => {
                    let matching: Vec<_> = engines
                        .iter()
                        .map(|e| e.as_ref())
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

        for dir in &[&user_dir, &shadow_dir] {
            for engine in &target_engines {
                let path = dir.join(format!(
                    "{}.{}",
                    name.as_str(),
                    engine.file_extension().as_str()
                ));
                if let Ok(source) = fs::read_to_string(&path)
                    && let Ok(module) = engine.load_module(name.as_str(), &source)
                {
                    return Ok(module);
                }
            }
        }

        let engine_suffix = match selection.as_explicit() {
            Some(id) => format!(" (engine: {})", id.as_str()),
            None => "".to_string(),
        };

        Err(BuiltinError::ModuleNotFound {
            module_name: name.clone(),
            engine_suffix,
        })
    }

    pub fn watch_scripts(
        command_tx: std::sync::Arc<dyn crate::features::module_runtime::ports::CommandSender>,
        app_env: &crate::shared::env::domain::AppEnvironment,
    ) -> Result<Box<dyn notify::Watcher>, BuiltinError> {
        use notify::{Event, RecursiveMode, Watcher};

        let home = app_env.home().as_path();
        let user_dir = PathBuf::from(home).join(".config/cranky/modules");
        let shadow_dir = PathBuf::from(home).join(".local/share/cranky/modules");

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res
                && event.kind.is_modify()
            {
                for path in event.paths {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        tracing::info!("Script modified: {:?}", path);
                        command_tx.send_command(crate::app::commands::AppCommand::ReloadModule(
                            crate::shared::primitives::ModuleName::new(stem),
                        ));
                    }
                }
            }
        })
        .map_err(|e| BuiltinError::Io(format!("Failed to create watcher: {}", e)))?;

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
        assert!(dir.join("hour.rhai").exists());
        assert!(dir.join("hour.lua").exists());
        assert!(dir.join("workspace.rhai").exists());
        assert!(dir.join("workspace.lua").exists());
        assert!(dir.join("applet.lua").exists());
        assert!(dir.join("applet.rhai").exists());
        assert!(dir.join("metrics.lua").exists());
        assert!(dir.join("metrics.rhai").exists());
    }

    #[test]
    fn test_find_module_applet_and_metrics_rhai() {
        use crate::shared::primitives::ModuleName;
        let env = get_test_env();
        let selection = EngineSelection::Explicit(EngineId::new("rhai"));
        let applet_mod =
            BuiltinModules::find_module(&ModuleName::new("applet"), &selection, &env).unwrap();
        let mut metrics_mod =
            BuiltinModules::find_module(&ModuleName::new("metrics"), &selection, &env).unwrap();
        let hub = crate::shared::events::signals::SignalHub::new(
            crate::shared::config::domain::Config::default(),
        );
        let metrics_state = crate::features::metrics::domain::MetricsState::new(
            crate::features::metrics::domain::CreateMetricsCommand::new(
                crate::features::metrics::domain::CpuUsage::new(50.0),
                vec![],
                crate::features::metrics::domain::MemoryBytes::new(1024),
                crate::features::metrics::domain::MemoryBytes::new(2048),
                crate::features::metrics::domain::MemoryBytes::new(0),
                crate::features::metrics::domain::MemoryBytes::new(0),
                vec![],
                crate::features::metrics::domain::NetworkSpeed::new(0),
                crate::features::metrics::domain::NetworkSpeed::new(0),
                crate::features::metrics::domain::Temperature::new(45.0),
                crate::features::metrics::domain::MetricsConfig::default(),
            ),
        );
        hub.metrics_tx().send(metrics_state).unwrap();
        metrics_mod.refresh(&hub, &[crate::shared::events::signals::SignalKind::Metrics]);

        let _ = applet_mod.render(&crate::shared::primitives::MonitorId::new("DP-1"));
        let metrics_node = metrics_mod.render(&crate::shared::primitives::MonitorId::new("DP-1"));
        assert!(matches!(
            metrics_node,
            crate::features::layout_engine::domain::LayoutNode::Flex { .. }
        ));
    }

    #[test]
    fn test_find_module_hour_rhai_format() {
        use crate::shared::primitives::ModuleName;
        let env = get_test_env();
        let selection = EngineSelection::Explicit(EngineId::new("rhai"));
        let mut hour_mod =
            BuiltinModules::find_module(&ModuleName::new("hour"), &selection, &env).unwrap();
        let mut options = std::collections::HashMap::new();
        options.insert(
            "format".to_string(),
            serde_json::Value::String("%H:%M".to_string()),
        );
        hour_mod
            .init(
                &crate::shared::config::domain::ModuleConfig::new(
                    ModuleName::new("hour"),
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
        hour_mod.refresh(&hub, &[crate::shared::events::signals::SignalKind::Time]);
        let node = hour_mod.render(&crate::shared::primitives::MonitorId::new("DP-1"));
        if let crate::features::layout_engine::domain::LayoutNode::Text { text, .. } = node {
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
            BuiltinModules::find_module(&ModuleName::new("hour"), &EngineSelection::Auto, &env);
        assert!(module.is_ok());
    }

    #[test]
    fn test_find_module_explicit_rhai() {
        use crate::shared::primitives::ModuleName;
        let env = get_test_env();
        let selection = EngineSelection::Explicit(EngineId::new("rhai"));
        let module = BuiltinModules::find_module(&ModuleName::new("hour"), &selection, &env);
        assert!(module.is_ok());
    }

    #[test]
    fn test_find_module_explicit_lua() {
        use crate::shared::primitives::ModuleName;
        let env = get_test_env();
        let selection = EngineSelection::Explicit(EngineId::new("lua"));
        let module = BuiltinModules::find_module(&ModuleName::new("hour"), &selection, &env);
        assert!(module.is_ok());
    }

    #[test]
    fn test_find_module_unsupported_engine() {
        use crate::shared::primitives::ModuleName;
        let env = get_test_env();
        let selection = EngineSelection::Explicit(EngineId::new("python"));
        let err = BuiltinModules::find_module(&ModuleName::new("hour"), &selection, &env)
            .err()
            .expect("Expected error");
        assert_eq!(
            err,
            BuiltinError::UnsupportedEngine {
                engine: "python".to_string(),
                module_name: ModuleName::new("hour"),
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
                engine_suffix: "".to_string(),
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
