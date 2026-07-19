#![allow(unsafe_code)]

use crate::domain::config::{BarConfig, ModuleConfig};
use crate::domain::signals::{SignalHub, SignalKind};
use crate::domain::{
    MonitorId,
};
use crate::modules::ModuleError;
use crate::ports::registry::AnyModulePort;
use rhai::{AST, Dynamic, Engine, Scope};
use std::sync::Mutex;

pub struct RhaiModule {
    engine: Mutex<Engine>,
    scope: Mutex<Scope<'static>>,
    ast: AST,
}

impl RhaiModule {
    pub fn new(name: String, source: &str) -> Result<Self, ModuleError> {
        let mut engine = Engine::new();

        engine.register_fn("exec", |cmd: String| {
            let _ = std::process::Command::new("sh").arg("-c").arg(&cmd).spawn();
        });

        let ast = engine.compile(source).map_err(|e| ModuleError::Internal {
            message: format!("Failed to compile Rhai script {}: {}", name, e),
        })?;

        Ok(Self {
            engine: Mutex::new(engine),
            scope: Mutex::new(Scope::new()),
            ast,
        })
    }

    pub fn external(name: &str) -> Option<Self> {
        let home = std::env::var("HOME").ok()?;
        let path = std::path::PathBuf::from(home)
            .join(".config/cranky/modules")
            .join(format!("{}.rhai", name));

        if path.exists() {
            let source = std::fs::read_to_string(path).ok()?;
            Self::new(name.to_string(), &source).ok()
        } else {
            None
        }
    }
}

impl AnyModulePort for RhaiModule {
    fn init(&mut self, config: &ModuleConfig, bar_config: &BarConfig) -> Result<(), String> {
        let mut scope = self.scope.lock().unwrap_or_else(|e| e.into_inner());
        let engine = self.engine.lock().unwrap_or_else(|e| e.into_inner());

        // Expose bar config
        let mut bar_map = rhai::Map::new();
        bar_map.insert(
            "font_family".into(),
            Dynamic::from(bar_config.font_family().as_str().to_string()),
        );
        bar_map.insert(
            "font_size".into(),
            Dynamic::from(bar_config.font_size().value()),
        );
        scope.push_constant("bar_config", bar_map);

        // Expose module config options
        let options_json = serde_json::to_string(config.options()).map_err(|e| e.to_string())?;
        let options_rhai: rhai::Map = engine
            .parse_json(&options_json, true)
            .map_err(|e| e.to_string())?;
        scope.push_constant("config", options_rhai);

        // Call init if it exists
        let _ = engine.call_fn::<()>(&mut scope, &self.ast, "init", ());
        Ok(())
    }

    fn subscriptions(&self) -> Vec<SignalKind> {
        let mut scope = self.scope.lock().unwrap_or_else(|e| e.into_inner());
        let engine = self.engine.lock().unwrap_or_else(|e| e.into_inner());

        if let Ok(subs) = engine.call_fn::<rhai::Array>(&mut scope, &self.ast, "subscriptions", ())
        {
            let mut result = Vec::new();
            for sub in subs {
                if let Some(s) = sub.try_cast::<String>() {
                    match s.as_str() {
                        "time" => result.push(SignalKind::Time),
                        "hyprland" => result.push(SignalKind::Hyprland),
                        "metrics" => result.push(SignalKind::Metrics),
                        _ => {}
                    }
                }
            }
            return result;
        }
        vec![]
    }

    fn refresh(&mut self, hub: &SignalHub, changed: &[SignalKind]) {
        let mut scope = self.scope.lock().unwrap_or_else(|e| e.into_inner());
        let engine = self.engine.lock().unwrap_or_else(|e| e.into_inner());

        if changed.contains(&SignalKind::Time) {
            let time = *hub.time_rx().borrow();
            scope.set_or_push("current_time", time.to_rfc3339());
        }

        if changed.contains(&SignalKind::Hyprland) {
            let hypr = hub.hyprland_rx().borrow().clone();
            if let Ok(hypr_json) = serde_json::to_string(&hypr)
                && let Ok(hypr_rhai) = engine.parse_json(&hypr_json, true)
            {
                scope.set_or_push("hyprland", hypr_rhai);
            }
        }

        if changed.contains(&SignalKind::Metrics) {
            let metrics = hub.metrics_rx().borrow().clone();
            if let Ok(metrics_json) = serde_json::to_string(&metrics)
                && let Ok(metrics_rhai) = engine.parse_json(&metrics_json, true)
            {
                scope.set_or_push("metrics", metrics_rhai);
            }
        }

        let _ = engine.call_fn::<()>(&mut scope, &self.ast, "refresh", ());
    }

    fn render(&self, monitor: &MonitorId) -> crate::domain::layout::LayoutNode {
        let mut scope = self.scope.lock().unwrap_or_else(|e| e.into_inner());
        let engine = self.engine.lock().unwrap_or_else(|e| e.into_inner());
        let monitor_id = monitor.as_str().to_string();

        match engine.call_fn::<rhai::Dynamic>(&mut scope, &self.ast, "render", (monitor_id,)) {
            Ok(result) => {
                match rhai::serde::from_dynamic::<crate::domain::layout::LayoutNode>(&result) {
                    Ok(node) => node,
                    Err(e) => {
                        tracing::error!("Failed to deserialize render output in rhai module: {}", e);
                        crate::domain::layout::LayoutNode::Flex { children: vec![], style: crate::domain::layout::FlexStyle::default(), background: None, radius: None, on_click: None, on_hover: None }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Module render error in rhai: {}", e);
                crate::domain::layout::LayoutNode::Flex { children: vec![], style: crate::domain::layout::FlexStyle::default(), background: None, radius: None, on_click: None, on_hover: None }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::{BarConfig, ModuleConfig};

    #[test]
    fn test_rhai_module_new_success() {
        let source = "
            fn init() {}
            fn subscriptions() { return [\"time\"]; }
            fn refresh() {}
            fn render(monitor) {
                return #{
                    type: \"flex\",
                    children: []
                };
            }
        ";
        let module = RhaiModule::new("test".into(), source);
        assert!(module.is_ok());
    }

    #[test]
    fn test_rhai_module_new_error() {
        let source = "fn invalid syntax(";
        let module = RhaiModule::new("test".into(), source);
        assert!(module.is_err());
    }

    #[test]
    fn test_rhai_module_lifecycle() {
        let source = "
            fn init() {}
            fn subscriptions() { return [\"time\", \"hyprland\", \"metrics\"]; }
            fn refresh() {}
            fn render(monitor) {
                return #{
                    type: \"flex\",
                    style: #{
                        direction: \"column\",
                    }
                };
            }
        ";
        let mut module = RhaiModule::new("test".into(), source).unwrap();
        
        let mod_config = ModuleConfig::new(
            "test".into(),
            true,
            std::collections::HashMap::new(),
        );
        let bar_config = BarConfig::default();
        
        assert!(module.init(&mod_config, &bar_config).is_ok());
        
        let subs = module.subscriptions();
        assert!(subs.contains(&SignalKind::Time));
        assert!(subs.contains(&SignalKind::Hyprland));
        assert!(subs.contains(&SignalKind::Metrics));
        
        let hub = SignalHub::new(
            crate::domain::config::Config::default()
        );
        module.refresh(&hub, &[SignalKind::Time]);
        
        let render_node = module.render(&MonitorId::new("DP-1"));
        if let crate::domain::layout::LayoutNode::Flex { style, .. } = render_node {
            assert_eq!(style.direction(), crate::domain::layout::FlexDirection::Column);
        } else {
            panic!("Expected Flex node");
        }
    }
}
