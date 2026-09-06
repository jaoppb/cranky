use super::helpers::monitor_info_to_rhai_map;
use super::metadata::{evaluate_metadata, setup_config_maps};
use super::module_impl::RhaiModule;
use super::sync::RhaiStateSynchronizer;
use crate::features::module_runtime::ports::{AnyModulePort, ModuleInitError};
use crate::shared::config::domain::ModuleConfig;
use crate::shared::events::signals::{SignalHub, SignalKind};
use crate::shared::primitives::{MonitorId, ScriptMonitorInfo};

impl AnyModulePort for RhaiModule {
    #[allow(clippy::significant_drop_tightening)]
    fn init(
        &mut self,
        config: &ModuleConfig,
        full_config: &crate::shared::config::domain::Config,
    ) -> Result<(), ModuleInitError> {
        let mut scope = self
            .scope
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let engine = self
            .engine
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        setup_config_maps(&mut scope, &engine, config, full_config)?;

        // Call init if it exists
        let _ = engine.call_fn::<()>(&mut scope, &self.ast, "init", ());

        let (subs, dbus_subs, styles) =
            evaluate_metadata(&engine, &mut scope, &self.ast, &self.name);
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

        RhaiStateSynchronizer::sync(
            &mut scope,
            &engine,
            &self.ast,
            hub,
            changed,
            &mut self.cached_monitors,
        );
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

        let mon_info = self
            .cached_monitors
            .iter()
            .find(|m| m.id() == monitor)
            .cloned()
            .unwrap_or_else(|| {
                ScriptMonitorInfo::new(
                    monitor.clone(),
                    monitor.as_str().to_string(),
                    crate::shared::primitives::geometry::Size::new(0, 0),
                    crate::shared::primitives::geometry::Scale::new(1.0),
                    false,
                    None,
                    None,
                )
            });

        let monitor_map = monitor_info_to_rhai_map(&mon_info);

        match engine.call_fn::<rhai::Dynamic>(&mut scope, &self.ast, "render", (monitor_map,)) {
            Ok(result) => {
                match rhai::serde::from_dynamic::<crate::features::vdom::domain::VNode>(&result) {
                    Ok(node) => node,
                    Err(e) => {
                        eprintln!("Failed to deserialize render output in rhai module: {e}");
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
                eprintln!("Module render error in rhai: {e:?}");
                crate::features::vdom::domain::VNode::new_flex(vec![], None, None, None, None, None)
            }
        }
    }

    #[allow(clippy::significant_drop_tightening)]
    fn call_function_with_args(
        &mut self,
        name: &crate::shared::primitives::FunctionName,
        args: &[&str],
    ) -> Result<(), ModuleInitError> {
        let mut scope = self
            .scope
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let engine = self
            .engine
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        if let Some(first_arg) = args.first() {
            let param_str = (*first_arg).to_string();
            match engine.call_fn::<rhai::Dynamic>(
                &mut scope,
                &self.ast,
                name.as_str(),
                (param_str,),
            ) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    if let rhai::EvalAltResult::ErrorFunctionNotFound(..) = &*e {
                        // Fallback to 0-argument function call for backwards compatibility
                    } else {
                        tracing::error!("Function call '{}' failed: {e}", name.as_str());
                        return Err(ModuleInitError::ScriptError(e.to_string()));
                    }
                }
            }
        }

        match engine.call_fn::<rhai::Dynamic>(&mut scope, &self.ast, name.as_str(), ()) {
            Ok(_) => Ok(()),
            Err(e) => {
                if let rhai::EvalAltResult::ErrorFunctionNotFound(f, ..) = &*e
                    && f == name.as_str()
                {
                    return Ok(());
                }
                tracing::error!("Function call '{}' failed: {e}", name.as_str());
                Err(ModuleInitError::ScriptError(e.to_string()))
            }
        }
    }
}
