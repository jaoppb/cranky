#![allow(unsafe_code)]

use crate::app::registry::ModuleError;
use crate::features::module_runtime::ports::AnyModulePort;
use crate::shared::config::domain::ModuleConfig;
use crate::shared::events::signals::{SignalHub, SignalKind};
use crate::shared::primitives::MonitorId;
use rhai::{AST, Dynamic, Engine, Scope};
use std::sync::Mutex;

pub struct RhaiModule {
    engine: Mutex<Engine>,
    scope: Mutex<Scope<'static>>,
    ast: AST,
    name: String,
    cached_subs: Vec<SignalKind>,
    cached_dbus_subs: Vec<crate::shared::dbus::domain::DBusSubscription>,
    cached_styles: Vec<crate::features::styling::domain::StyleSheetName>,
}

impl RhaiModule {
    /// # Errors
    ///
    /// Returns `ModuleError::Internal` if compiling or evaluating the script fails.
    pub fn new(name: String, source: &str) -> Result<Self, ModuleError> {
        let mut engine = Engine::new();
        engine.set_max_expr_depths(0, 0);

        engine.register_fn("exec", |cmd: String| {
            let _ = std::process::Command::new("sh").arg("-c").arg(&cmd).spawn();
        });

        let ast = engine.compile(source).map_err(|e| ModuleError::Internal {
            message: format!("Failed to compile Rhai script {name}: {e}"),
        })?;

        let mut scope = Scope::new();
        scope.push("config", rhai::Map::new());
        scope.push("bar_config", rhai::Map::new());
        scope.push("current_time", rhai::Dynamic::UNIT);
        scope.push("hyprland", rhai::Dynamic::UNIT);
        scope.push("systray", rhai::Dynamic::UNIT);
        scope.push("metrics", rhai::Dynamic::UNIT);
        scope.push("dbus", rhai::Dynamic::UNIT);

        if let Err(e) = engine.run_ast_with_scope(&mut scope, &ast) {
            return Err(ModuleError::Internal {
                message: format!("Failed to initialize Rhai script scope {name}: {e}"),
            });
        }

        Ok(Self {
            engine: Mutex::new(engine),
            scope: Mutex::new(scope),
            ast,
            name,
            cached_subs: Vec::new(),
            cached_dbus_subs: Vec::new(),
            cached_styles: Vec::new(),
        })
    }

    fn evaluate_metadata(
        engine: &Engine,
        scope: &mut Scope<'static>,
        ast: &AST,
        module_name: &str,
    ) -> (
        Vec<SignalKind>,
        Vec<crate::shared::dbus::domain::DBusSubscription>,
        Vec<crate::features::styling::domain::StyleSheetName>,
    ) {
        let mut subs = Vec::new();
        let mut dbus_subs = Vec::new();
        let mut styles = Vec::new();

        if let Ok(meta) = engine.call_fn::<rhai::Map>(scope, ast, "metadata", ()) {
            if let Some(subs_arr) = meta
                .get("subscriptions")
                .and_then(|v| v.clone().try_cast::<rhai::Array>())
            {
                Self::parse_subscriptions_array(&subs_arr, &mut subs, &mut dbus_subs);
            }
            if let Some(styles_arr) = meta
                .get("styles")
                .and_then(|v| v.clone().try_cast::<rhai::Array>())
            {
                for s in styles_arr {
                    if let Some(str_val) = s.try_cast::<String>()
                        && let Ok(sheet) =
                            crate::features::styling::domain::StyleSheetName::new(str_val)
                    {
                        styles.push(sheet);
                    }
                }
            }
        } else if let Ok(subs_arr) = engine.call_fn::<rhai::Array>(scope, ast, "subscriptions", ())
        {
            Self::parse_subscriptions_array(&subs_arr, &mut subs, &mut dbus_subs);
        }

        if styles.is_empty()
            && let Ok(default_sheet) =
                crate::features::styling::domain::StyleSheetName::new(module_name)
        {
            styles.push(default_sheet);
        }

        (subs, dbus_subs, styles)
    }

    fn parse_dbus_subscription(
        map: &rhai::Map,
    ) -> Option<crate::shared::dbus::domain::DBusSubscription> {
        let is_dbus = map
            .get("type")
            .and_then(|v| v.clone().try_cast::<String>())
            .as_deref()
            == Some("dbus");
        if !is_dbus {
            return None;
        }

        let bus = match map
            .get("bus")
            .and_then(|v| v.clone().try_cast::<String>())
            .as_deref()
        {
            Some("system") => crate::shared::dbus::domain::BusType::System,
            _ => crate::shared::dbus::domain::BusType::Session,
        };

        Some(crate::shared::dbus::domain::DBusSubscription::new(
            bus,
            map.get("destination")
                .and_then(|v| v.clone().try_cast::<String>())
                .map(crate::shared::dbus::domain::Destination::new),
            map.get("path")
                .and_then(|v| v.clone().try_cast::<String>())
                .map(crate::shared::dbus::domain::Path::new),
            map.get("interface")
                .and_then(|v| v.clone().try_cast::<String>())
                .map(crate::shared::dbus::domain::Interface::new),
            map.get("member")
                .and_then(|v| v.clone().try_cast::<String>())
                .map(crate::shared::dbus::domain::Member::new),
        ))
    }

    fn parse_subscriptions_array(
        subs: &rhai::Array,
        result: &mut Vec<SignalKind>,
        dbus_subs: &mut Vec<crate::shared::dbus::domain::DBusSubscription>,
    ) {
        for sub in subs {
            if let Some(s) = sub.clone().try_cast::<String>() {
                match s.as_str() {
                    "time" => result.push(SignalKind::Time),
                    "hyprland" => result.push(SignalKind::Hyprland),
                    "systray" => result.push(SignalKind::Systray),
                    "metrics" => result.push(SignalKind::Metrics),
                    "mpris" => result.push(SignalKind::Mpris),
                    _ => {}
                }
            } else if let Some(map) = sub.clone().try_cast::<rhai::Map>()
                && let Some(dbus_sub) = Self::parse_dbus_subscription(&map)
            {
                result.push(SignalKind::DBus);
                dbus_subs.push(dbus_sub);
            }
        }
    }
}

impl AnyModulePort for RhaiModule {
    #[allow(clippy::significant_drop_tightening)]
    fn init(
        &mut self,
        config: &ModuleConfig,
        full_config: &crate::shared::config::domain::Config,
    ) -> Result<(), crate::features::module_runtime::ports::ModuleInitError> {
        use crate::features::module_runtime::ports::ModuleInitError;

        let root_config = full_config.root();
        let mut scope = self
            .scope
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let engine = self
            .engine
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        // Expose root config
        let mut root_map = rhai::Map::new();
        root_map.insert(
            "name".into(),
            Dynamic::from(root_config.name().as_str().to_string()),
        );
        root_map.insert(
            "height".into(),
            Dynamic::from(i64::from(root_config.height().value())),
        );
        scope.set_or_push("root_config", root_map);

        // Expose module config options
        let options_json = serde_json::to_string(config.options())
            .map_err(|e| ModuleInitError::ConfigError(e.to_string()))?;
        let options_rhai: rhai::Map = engine
            .parse_json(&options_json, true)
            .map_err(|e| ModuleInitError::ScriptError(e.to_string()))?;
        scope.set_or_push("config", options_rhai);

        // Call init if it exists
        let _ = engine.call_fn::<()>(&mut scope, &self.ast, "init", ());

        let (subs, dbus_subs, styles) =
            Self::evaluate_metadata(&engine, &mut scope, &self.ast, &self.name);
        self.cached_subs = subs;
        self.cached_dbus_subs = dbus_subs;
        self.cached_styles = styles;

        Ok(())
    }

    fn subscriptions(&self) -> &[SignalKind] {
        &self.cached_subs
    }

    fn dbus_subscriptions(&self) -> &[crate::shared::dbus::domain::DBusSubscription] {
        &self.cached_dbus_subs
    }

    fn styles(&self) -> &[crate::features::styling::domain::StyleSheetName] {
        &self.cached_styles
    }

    #[allow(clippy::significant_drop_tightening)]
    fn refresh(&mut self, hub: &SignalHub, changed: &[SignalKind]) {
        let mut scope = self
            .scope
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let engine = self
            .engine
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

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

        if changed.contains(&SignalKind::Systray) {
            let systray = hub.systray_rx().borrow().clone();
            let items = systray.items().values().collect::<Vec<_>>();
            if let Ok(systray_json) = serde_json::to_string(&items)
                && let Ok(systray_rhai) = engine.parse_json(&systray_json, true)
            {
                scope.set_or_push("systray", systray_rhai);
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

        if changed.contains(&SignalKind::Mpris) {
            let mpris = hub.mpris_rx().borrow().clone();
            if let Ok(mpris_json) = serde_json::to_string(&mpris)
                && let Ok(mpris_rhai) = engine.parse_json(&mpris_json, true)
            {
                scope.set_or_push("mpris", mpris_rhai);
            }
        }

        let mut dbus_handled = false;
        for signal in changed {
            if matches!(signal, SignalKind::DBus) && !dbus_handled {
                let dbus_state = hub.dbus_rx().borrow().clone();
                if let Ok(dbus_json) = serde_json::to_string(&dbus_state.properties())
                    && let Ok(dbus_rhai) = engine.parse_json(&dbus_json, true)
                {
                    scope.set_or_push("dbus", dbus_rhai);
                }
                dbus_handled = true;
            }
        }

        if let Err(e) = engine.call_fn::<()>(&mut scope, &self.ast, "refresh", ()) {
            tracing::error!("Rhai refresh error: {e}");
        }
    }

    #[allow(clippy::significant_drop_tightening)]
    fn render(&self, monitor: &MonitorId) -> crate::features::vdom::domain::VNode {
        let mut scope = self
            .scope
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let engine = self
            .engine
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let monitor_id = monitor.as_str().to_string();

        match engine.call_fn::<rhai::Dynamic>(&mut scope, &self.ast, "render", (monitor_id,)) {
            Ok(result) => {
                match rhai::serde::from_dynamic::<crate::features::vdom::domain::VNode>(&result) {
                    Ok(node) => node,
                    Err(e) => {
                        tracing::error!("Failed to deserialize render output in rhai module: {e}");
                        crate::features::vdom::domain::VNode::new_flex(
                            vec![],
                            None,
                            None,
                            None,
                            None,
                            None,
                        )
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Module render error in rhai: {e}");
                crate::features::vdom::domain::VNode::new_flex(vec![], None, None, None, None, None)
            }
        }
    }

    #[allow(clippy::significant_drop_tightening)]
    fn call_function(
        &mut self,
        name: &crate::shared::primitives::FunctionName,
    ) -> Result<(), crate::features::module_runtime::ports::ModuleInitError> {
        let mut scope = self
            .scope
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let engine = self
            .engine
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        match engine.call_fn::<()>(&mut scope, &self.ast, name.as_str(), ()) {
            Ok(()) => Ok(()),
            Err(e) => {
                tracing::error!("Function call '{}' failed: {e}", name.as_str());
                Err(
                    crate::features::module_runtime::ports::ModuleInitError::ScriptError(
                        e.to_string(),
                    ),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::events::signals::SignalKind;

    #[test]
    fn test_rhai_module_new_success() {
        let source = "
            fn subscriptions() { return [\"time\"]; }
            fn refresh() {}
            fn render(monitor) {
                return #{
                    type: \"flex\",
                    children: [
                        #{ type: \"text\", text: \"hello\" }
                    ]
                };
            }
        ";
        let module = RhaiModule::new("test_mod".into(), source);
        assert!(module.is_ok());
    }

    #[test]
    fn test_rhai_module_new_error() {
        let source = "this is not valid rhai syntax !!!";
        let module = RhaiModule::new("test_err".into(), source);
        assert!(module.is_err());
    }

    #[test]
    fn test_rhai_module_lifecycle() {
        let source = "
            fn subscriptions() { return [\"time\", \"hyprland\", \"metrics\"]; }
            fn refresh() {}
            fn render(monitor) {
                return #{
                    type: \"flex\",
                    children: []
                };
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
    fn test_rhai_module_top_level_variables() {
        let source = "
            let greeting = \"Hello, World!\";
            fn refresh() {
                greeting = \"Hello, Rhai!\";
            }
            fn get_text(g) {
                g
            }
            fn render(monitor) {
                return #{
                    type: \"text\",
                    text: get_text(greeting),
                    color: \"#ffffff\",
                };
            }
        ";
        let mut module = RhaiModule::new("test_vars".into(), source).unwrap();
        let hub = SignalHub::new(crate::shared::config::domain::Config::default());
        module.refresh(&hub, &[]);
        let render_node = module.render(&MonitorId::new("DP-1"));
        assert_eq!(
            render_node.tag(),
            crate::features::vdom::domain::NodeTag::Text
        );
        if let crate::features::vdom::domain::VNodeKind::Text { text } = render_node.kind() {
            assert_eq!(text.as_str(), "Hello, Rhai!");
        } else {
            panic!("Expected Text node");
        }
    }
}
