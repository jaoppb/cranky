mod render;

use super::helpers::monitor_info_to_rhai_map;
use super::metadata::{evaluate_metadata, setup_config_maps};
use super::module_impl::RhaiModule;
use super::script::RhaiScript;
use super::sync::RhaiStateSynchronizer;
use crate::features::module_runtime::ports::{AnyModulePort, ModuleInitError};
use crate::features::vdom::domain::VNode;
use crate::shared::config::domain::ModuleConfig;
use crate::shared::events::signals::{SignalHub, SignalKind};
use crate::shared::primitives::MonitorId;
use render::node_from_render_output;
use rhai::{Dynamic, Engine, EvalAltResult, Scope};
use std::sync::MutexGuard;

impl AnyModulePort for RhaiModule {
    fn init(
        &mut self,
        config: &ModuleConfig,
        full_config: &crate::shared::config::domain::Config,
    ) -> Result<(), ModuleInitError> {
        let (subs, dbus_subs, styles) = {
            let (mut scope, engine) = self.locked_state();
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
        let (mut scope, engine) = self.locked_state();
        let monitors = RhaiStateSynchronizer::sync(
            &mut RhaiScript::new(&engine, &mut scope, &self.ast),
            hub,
            changed,
        );
        drop(scope);
        drop(engine);
        self.cached_monitors = monitors;
    }

    fn render(&self, monitor: &MonitorId) -> VNode {
        let monitor_map = monitor_info_to_rhai_map(&self.monitor_for_render(monitor));

        let call_res = {
            let (mut scope, engine) = self.locked_state();
            let res = engine.call_fn::<Dynamic>(&mut scope, &self.ast, "render", (monitor_map,));
            drop(scope);
            drop(engine);
            res
        };

        node_from_render_output(call_res)
    }

    fn call_function_with_args(
        &mut self,
        name: &crate::shared::primitives::FunctionName,
        args: &[&str],
    ) -> Result<(), ModuleInitError> {
        let res = {
            let (mut scope, engine) = self.locked_state();
            let mut script = RhaiScript::new(&engine, &mut scope, &self.ast);
            let call_res =
                call_with_first_argument(&mut script, name.as_str(), args.first().copied());
            drop(scope);
            drop(engine);
            call_res
        };

        res.or_else(|error| ignore_missing_function(&error, name))
    }
}

impl RhaiModule {
    /// The module's scope and engine, locked in that order; a lock poisoned by a panic in another
    /// call is taken over.
    fn locked_state(&self) -> (MutexGuard<'_, Scope<'static>>, MutexGuard<'_, Engine>) {
        let scope = self
            .scope
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let engine = self
            .engine
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        (scope, engine)
    }
}

/// Calls the script function `name` with `first_arg` as a string when given, and without
/// arguments when there is none or the script defines `name` only without one.
fn call_with_first_argument(
    script: &mut RhaiScript<'_>,
    name: &str,
    first_arg: Option<&str>,
) -> Result<(), Box<EvalAltResult>> {
    let (engine, scope, ast) = script.parts();
    let Some(first_arg) = first_arg else {
        return engine.call_fn::<Dynamic>(scope, ast, name, ()).map(|_| ());
    };
    match engine.call_fn::<Dynamic>(scope, ast, name, (first_arg.to_string(),)) {
        Ok(_) => Ok(()),
        Err(error) if matches!(*error, EvalAltResult::ErrorFunctionNotFound(..)) => {
            engine.call_fn::<Dynamic>(scope, ast, name, ()).map(|_| ())
        }
        Err(error) => Err(error),
    }
}

/// `Ok` when `error` only says the script defines no function `name`, as such a call does nothing;
/// otherwise the error, logged.
fn ignore_missing_function(
    error: &EvalAltResult,
    name: &crate::shared::primitives::FunctionName,
) -> Result<(), ModuleInitError> {
    if let EvalAltResult::ErrorFunctionNotFound(function_name, ..) = error
        && function_name == name.as_str()
    {
        return Ok(());
    }
    tracing::error!("Function call '{}' failed: {error}", name.as_str());
    Err(ModuleInitError::ScriptError(error.to_string()))
}

/// Logs a render whose value does not deserialize into a node.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_render_output_rejected(error: &EvalAltResult) {
    tracing::error!("Failed to deserialize render output in rhai module: {error}");
}

/// Logs a failed `render` call.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_render_failed(error: &EvalAltResult) {
    tracing::error!("Module render error in rhai: {error:?}");
}
