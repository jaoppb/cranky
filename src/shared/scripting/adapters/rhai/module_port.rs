use super::helpers::monitor_info_to_rhai_map;
use super::metadata::{evaluate_metadata, setup_config_maps};
use super::module_impl::RhaiModule;
use super::script::RhaiScript;
use super::sync::RhaiStateSynchronizer;
use crate::features::module_runtime::ports::{AnyModulePort, ModuleInitError};
use crate::shared::config::domain::ModuleConfig;
use crate::shared::events::signals::{SignalHub, SignalKind};
use crate::shared::primitives::{MonitorId, ScriptMonitorInfo};

impl AnyModulePort for RhaiModule {
    fn init(
        &mut self,
        config: &ModuleConfig,
        full_config: &crate::shared::config::domain::Config,
    ) -> Result<(), ModuleInitError> {
        let (subs, dbus_subs, styles) = {
            let mut scope = self
                .scope
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let engine = self
                .engine
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            let mut script = RhaiScript::new(&engine, &mut scope, &self.ast);
            setup_config_maps(&mut script, config, full_config)?;

            // Call init if it exists
            let (script_engine, script_scope, ast) = script.parts();
            let _ = script_engine.call_fn::<()>(script_scope, ast, "init", ());

            let meta = evaluate_metadata(&mut script, &self.name);
            drop(scope);
            drop(engine);
            meta
        };

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

    fn refresh(&mut self, hub: &SignalHub, changed: &[SignalKind]) {
        let mut scope = self
            .scope
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let engine = self
            .engine
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        self.cached_monitors = RhaiStateSynchronizer::sync(
            &mut RhaiScript::new(&engine, &mut scope, &self.ast),
            hub,
            changed,
        );
        drop(scope);
        drop(engine);
    }

    fn render(&self, monitor: &MonitorId) -> crate::features::vdom::domain::VNode {
        let mon_info = self
            .cached_monitors
            .iter()
            .find(|monitor_info| monitor_info.id() == monitor)
            .cloned()
            .unwrap_or_else(|| ScriptMonitorInfo::from_id(monitor));

        let monitor_map = monitor_info_to_rhai_map(&mon_info);

        let call_res = {
            let mut scope = self
                .scope
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let engine = self
                .engine
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            let res =
                engine.call_fn::<rhai::Dynamic>(&mut scope, &self.ast, "render", (monitor_map,));
            drop(scope);
            drop(engine);
            res
        };

        match call_res {
            Ok(result) => {
                match rhai::serde::from_dynamic::<crate::features::vdom::domain::VNode>(&result) {
                    Ok(node) => node,
                    Err(error) => {
                        tracing::error!(
                            "Failed to deserialize render output in rhai module: {error}"
                        );
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
            Err(error) => {
                tracing::error!("Module render error in rhai: {error:?}");
                crate::features::vdom::domain::VNode::new_flex(vec![], None, None, None, None, None)
            }
        }
    }

    fn call_function_with_args(
        &mut self,
        name: &crate::shared::primitives::FunctionName,
        args: &[&str],
    ) -> Result<(), ModuleInitError> {
        let res = {
            let mut scope = self
                .scope
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let engine = self
                .engine
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            let call_res = if let Some(first_arg) = args.first() {
                let param_str = (*first_arg).to_string();
                let first_res = engine.call_fn::<rhai::Dynamic>(
                    &mut scope,
                    &self.ast,
                    name.as_str(),
                    (param_str,),
                );
                match first_res {
                    Ok(_) => Ok(()),
                    Err(error) => {
                        if let rhai::EvalAltResult::ErrorFunctionNotFound(..) = &*error {
                            engine
                                .call_fn::<rhai::Dynamic>(&mut scope, &self.ast, name.as_str(), ())
                                .map(|_| ())
                        } else {
                            Err(error)
                        }
                    }
                }
            } else {
                engine
                    .call_fn::<rhai::Dynamic>(&mut scope, &self.ast, name.as_str(), ())
                    .map(|_| ())
            };

            drop(scope);
            drop(engine);
            call_res
        };

        match res {
            Ok(()) => Ok(()),
            Err(error) => {
                if let rhai::EvalAltResult::ErrorFunctionNotFound(function_name, ..) = &*error
                    && function_name == name.as_str()
                {
                    return Ok(());
                }
                tracing::error!("Function call '{}' failed: {error}", name.as_str());
                Err(ModuleInitError::ScriptError(error.to_string()))
            }
        }
    }
}
