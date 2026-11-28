mod render;

use super::metadata::{evaluate_metadata, setup_config_tables};
use super::module_impl::LuaModule;
use super::sync::LuaStateSynchronizer;
use crate::features::module_runtime::ports::{AnyModulePort, ModuleInitError};
use crate::features::vdom::domain::VNode;
use crate::shared::config::domain::ModuleConfig;
use crate::shared::dbus::domain::DBusSubscription;
use crate::shared::events::signals::{SignalHub, SignalKind};
use crate::shared::primitives::MonitorId;
use mlua::Lua;
use render::{call_render, empty_flex};
use std::sync::MutexGuard;
use std::time::Instant;

impl LuaModule {
    /// The module's Lua state, locked; a lock poisoned by a panic in another call is taken over.
    fn locked_state(&self) -> MutexGuard<'_, Lua> {
        self.lua
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Runs the module's source in `lua`, then its `init` when the script defines one.
    fn run_source_and_init(&self, lua: &Lua) -> Result<(), ModuleInitError> {
        lua.load(&self.source)
            .set_name(&self.name)
            .exec()
            .map_err(|error| {
                ModuleInitError::ScriptError(format!("Lua load error in {}: {error}", self.name))
            })?;
        let Ok(init_fn) = lua.globals().get::<mlua::Function>("init") else {
            return Ok(());
        };
        init_fn.call::<()>(()).map_err(|error| {
            ModuleInitError::ScriptError(format!("Lua init error in {}: {error}", self.name))
        })
    }
}

impl AnyModulePort for LuaModule {
    fn init(
        &mut self,
        config: &ModuleConfig,
        full_config: &crate::shared::config::domain::Config,
    ) -> Result<(), ModuleInitError> {
        let (subs, dbus_subs, styles) = {
            let lua = self.locked_state();
            setup_config_tables(&lua, config, full_config)?;
            self.run_source_and_init(&lua)?;
            let meta = evaluate_metadata(&lua, &self.name);
            drop(lua);
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

    fn dbus_subscriptions(&self) -> &[DBusSubscription] {
        &self.cached_dbus_subs
    }

    fn styles(&self) -> &[crate::features::styling::domain::StyleSheetName] {
        &self.cached_styles
    }

    fn refresh(&mut self, hub: &SignalHub, changed: &[SignalKind]) {
        let start = Instant::now();
        log_refreshing(changed);
        let lua = self.locked_state();
        match LuaStateSynchronizer::sync(&lua, hub, changed) {
            Ok(()) => log_refresh_completed(changed, start),
            Err(error) => log_refresh_failed(changed, &error),
        }
    }

    fn render(&self, monitor: &MonitorId) -> VNode {
        let start = Instant::now();
        let render_result = {
            let lua = self.locked_state();
            let res = call_render(&lua, monitor);
            drop(lua);
            res
        };

        match render_result {
            Ok(Ok(Ok(node))) => {
                log_render_completed(&self.name, monitor, start);
                return node;
            }
            Ok(Ok(Err(error))) => log_render_output_rejected(&self.name, monitor, &error),
            Ok(Err(error)) => log_render_failed(&self.name, monitor, &error),
            Err(_) => {}
        }

        empty_flex()
    }

    fn call_function_with_args(
        &mut self,
        name: &crate::shared::primitives::FunctionName,
        args: &[&str],
    ) -> Result<(), ModuleInitError> {
        let res = {
            let lua = self.locked_state();
            let globals = lua.globals();

            let call_res = globals
                .get::<mlua::Function>(name.as_str())
                .map(|func| call_with_first_arg(&func, args));
            drop(globals);
            drop(lua);
            call_res
        };

        res.unwrap_or(Ok(())).map_err(|error| {
            ModuleInitError::ScriptError(format!("Failed to call function '{name}': {error}"))
        })
    }
}

/// Calls `func` with the first of `args`, or with no argument when `args` is empty.
fn call_with_first_arg(func: &mlua::Function, args: &[&str]) -> mlua::Result<()> {
    args.first()
        .map_or_else(|| func.call::<()>(()), |arg0| func.call::<()>(*arg0))
}

/// Logs a refresh of `changed` starting.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_refreshing(changed: &[SignalKind]) {
    tracing::debug!(?changed, "Refreshing LuaModulePort");
}

/// Logs a refresh that published `changed` and ran the script, with how long it took since `start`.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_refresh_completed(changed: &[SignalKind], start: Instant) {
    tracing::debug!(
        ?changed,
        duration_ms = start.elapsed().as_millis(),
        duration_micros = start.elapsed().as_micros(),
        "LuaModulePort refresh completed successfully"
    );
}

/// Logs a refresh of `changed` that failed with `error`.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_refresh_failed(changed: &[SignalKind], error: &mlua::Error) {
    tracing::error!(?changed, err = ?error, "LuaModulePort refresh failed");
}

/// Logs a render of module `name` on `monitor`, with how long it took since `start`.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_render_completed(name: &str, monitor: &MonitorId, start: Instant) {
    tracing::debug!(
        module = %name,
        monitor = %monitor,
        duration_ms = start.elapsed().as_millis(),
        duration_micros = start.elapsed().as_micros(),
        "Lua render completed successfully"
    );
}

/// Logs a render of module `name` on `monitor` whose value is no node.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_render_output_rejected(name: &str, monitor: &MonitorId, error: &mlua::Error) {
    tracing::error!(
        module = %name,
        monitor = %monitor,
        err = ?error,
        "Failed to convert Lua return value to VNode"
    );
}

/// Logs a render of module `name` on `monitor` whose `render` call failed.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_render_failed(name: &str, monitor: &MonitorId, error: &mlua::Error) {
    tracing::error!(
        module = %name,
        monitor = %monitor,
        err = ?error,
        "Lua render_fn execution failed"
    );
}
