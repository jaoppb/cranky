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
        module_name: String,
    },
    #[error("Module '{module_name}' not found")]
    ModuleNotFound {
        module_name: String,
    },
}

pub struct BuiltinModules;

impl BuiltinModules {
    const BUILTINS: &[(&'static str, &'static str)] = &[
        ("hour.lua", include_str!("../../assets/widgets/hour.lua")),
        ("hour.rhai", include_str!("../../assets/widgets/hour.rhai")),
        ("workspace.lua", include_str!("../../assets/widgets/workspace.lua")),
        ("workspace.rhai", include_str!("../../assets/widgets/workspace.rhai")),
        ("applet.lua", include_str!("../../assets/widgets/applet.lua")),
        ("applet.rhai", include_str!("../../assets/widgets/applet.rhai")),
        ("metrics.lua", include_str!("../../assets/widgets/metrics.lua")),
        ("metrics.rhai", include_str!("../../assets/widgets/metrics.rhai")),
    ];

    pub fn ensure_builtins() -> Result<PathBuf, BuiltinError> {
        let home = std::env::var("HOME").map_err(|e| BuiltinError::Env(e.to_string()))?;
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
        name: &str,
        selection: &crate::shared::config::domain::EngineSelection,
    ) -> Result<Box<dyn AnyModulePort>, BuiltinError> {
        let _ = Self::ensure_builtins()?;

        let home = std::env::var("HOME").map_err(|e| BuiltinError::Env(e.to_string()))?;
        let user_dir = PathBuf::from(&home).join(".config/cranky/modules");
        let shadow_dir = PathBuf::from(&home).join(".local/share/cranky/modules");

        let engines = Self::registered_engines();

        let target_engines: Vec<&dyn crate::shared::scripting::ports::ScriptEnginePort> = match selection {
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
                        module_name: name.to_string(),
                    });
                }
                matching
            }
        };

        for dir in &[&user_dir, &shadow_dir] {
            for engine in &target_engines {
                let path = dir.join(format!("{}.{}", name, engine.file_extension().as_str()));
                if let Ok(source) = fs::read_to_string(&path)
                    && let Ok(module) = engine.load_module(name, &source)
                {
                    return Ok(module);
                }
            }
        }

        let err_name = match selection.as_explicit() {
            Some(id) => format!("{} (engine: {})", name, id.as_str()),
            None => name.to_string(),
        };

        Err(BuiltinError::ModuleNotFound {
            module_name: err_name,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::config::domain::{EngineId, EngineSelection};

    #[test]
    fn test_ensure_builtins() {
        let dir = BuiltinModules::ensure_builtins().expect("ensure_builtins failed");
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
        let selection = EngineSelection::Explicit(EngineId::new("rhai"));
        let applet_mod = BuiltinModules::find_module("applet", &selection).unwrap();
        let mut metrics_mod = BuiltinModules::find_module("metrics", &selection).unwrap();
        let hub = crate::shared::events::signals::SignalHub::new(crate::shared::config::domain::Config::default());
        let metrics_state = crate::features::metrics::domain::MetricsState::new(
            crate::features::metrics::domain::CreateMetricsCommand {
                cpu_usage: crate::features::metrics::domain::CpuUsage::new(50.0),
                per_core: vec![],
                memory_used: crate::features::metrics::domain::MemoryBytes::new(1024),
                memory_total: crate::features::metrics::domain::MemoryBytes::new(2048),
                swap_used: crate::features::metrics::domain::MemoryBytes::new(0),
                swap_total: crate::features::metrics::domain::MemoryBytes::new(0),
                disks: vec![],
                network_tx: crate::features::metrics::domain::NetworkSpeed::new(0),
                network_rx: crate::features::metrics::domain::NetworkSpeed::new(0),
                temperature: crate::features::metrics::domain::Temperature::new(45.0),
                config: crate::features::metrics::domain::MetricsConfig::default(),
            },
        );
        hub.metrics_tx().send(metrics_state).unwrap();
        metrics_mod.refresh(&hub, &[crate::shared::events::signals::SignalKind::Metrics]);

        let _ = applet_mod.render(&crate::shared::primitives::MonitorId::new("DP-1"));
        let metrics_node = metrics_mod.render(&crate::shared::primitives::MonitorId::new("DP-1"));
        assert!(matches!(metrics_node, crate::features::layout_engine::domain::LayoutNode::Flex { .. }));
    }

    #[test]
    fn test_find_module_hour_rhai_format() {
        let selection = EngineSelection::Explicit(EngineId::new("rhai"));
        let mut hour_mod = BuiltinModules::find_module("hour", &selection).unwrap();
        let mut options = std::collections::HashMap::new();
        options.insert("format".to_string(), serde_json::Value::String("%H:%M".to_string()));
        hour_mod.init(
            &crate::shared::config::domain::ModuleConfig::new("hour".to_string(), true, selection.clone(), options),
            &crate::shared::config::domain::Config::default(),
        ).unwrap();
        let hub = crate::shared::events::signals::SignalHub::new(crate::shared::config::domain::Config::default());
        let test_time = chrono::DateTime::parse_from_rfc3339("2026-07-28T14:30:45+00:00").unwrap().with_timezone(&chrono::Local);
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
        let module = BuiltinModules::find_module("hour", &EngineSelection::Auto);
        assert!(module.is_ok());
    }

    #[test]
    fn test_find_module_explicit_rhai() {
        let selection = EngineSelection::Explicit(EngineId::new("rhai"));
        let module = BuiltinModules::find_module("hour", &selection);
        assert!(module.is_ok());
    }

    #[test]
    fn test_find_module_explicit_lua() {
        let selection = EngineSelection::Explicit(EngineId::new("lua"));
        let module = BuiltinModules::find_module("hour", &selection);
        assert!(module.is_ok());
    }

    #[test]
    fn test_find_module_unsupported_engine() {
        let selection = EngineSelection::Explicit(EngineId::new("python"));
        let err = BuiltinModules::find_module("hour", &selection)
            .err()
            .expect("Expected error");
        assert_eq!(
            err,
            BuiltinError::UnsupportedEngine {
                engine: "python".to_string(),
                module_name: "hour".to_string(),
            }
        );
    }

    #[test]
    fn test_find_module_not_found() {
        let err = BuiltinModules::find_module("nonexistent_module_test", &EngineSelection::Auto)
            .err()
            .expect("Expected error");
        assert_eq!(
            err,
            BuiltinError::ModuleNotFound {
                module_name: "nonexistent_module_test".to_string(),
            }
        );
    }

    #[test]
    fn test_find_module_not_found_with_engine() {
        let selection = EngineSelection::Explicit(EngineId::new("rhai"));
        let err = BuiltinModules::find_module("nonexistent_module_test", &selection)
            .err()
            .expect("Expected error");
        assert_eq!(
            err,
            BuiltinError::ModuleNotFound {
                module_name: "nonexistent_module_test (engine: rhai)".to_string(),
            }
        );
    }

    #[test]
    fn test_builtin_error_display() {
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
                module_name: "mod".into(),
            }
            .to_string(),
            "Unsupported engine 'py' for module 'mod' (expected 'rhai' or 'lua')"
        );
        assert_eq!(
            BuiltinError::ModuleNotFound {
                module_name: "mod".into(),
            }
            .to_string(),
            "Module 'mod' not found"
        );
    }
}

