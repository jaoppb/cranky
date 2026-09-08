use super::metadata::{evaluate_metadata, setup_config_tables};
use super::module_impl::LuaModule;
use super::sync::LuaStateSynchronizer;
use super::userdata::LuaMonitor;
use super::vnode_parser::value_to_vnode;
use crate::features::module_runtime::ports::{AnyModulePort, ModuleInitError};
use crate::shared::config::domain::ModuleConfig;
use crate::shared::dbus::domain::DBusSubscription;
use crate::shared::events::signals::{SignalHub, SignalKind};
use crate::shared::primitives::{MonitorId, ScriptMonitorInfo};

impl AnyModulePort for LuaModule {
    fn init(
        &mut self,
        config: &ModuleConfig,
        full_config: &crate::shared::config::domain::Config,
    ) -> Result<(), ModuleInitError> {
        let (subs, dbus_subs, styles) = {
            let lua = self
                .lua
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let globals = lua.globals();

            setup_config_tables(&lua, config, full_config)?;

            lua.load(&self.source)
                .set_name(&self.name)
                .exec()
                .map_err(|e| {
                    ModuleInitError::ScriptError(format!("Lua load error in {}: {e}", self.name))
                })?;

            if let Ok(init_fn) = globals.get::<mlua::Function>("init") {
                init_fn.call::<()>(()).map_err(|e| {
                    ModuleInitError::ScriptError(format!("Lua init error in {}: {e}", self.name))
                })?;
            }

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
        let t0 = std::time::Instant::now();
        tracing::debug!(?changed, "Refreshing LuaModulePort");
        let lua = self
            .lua
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match LuaStateSynchronizer::sync(&lua, hub, changed) {
            Ok(()) => {
                tracing::debug!(
                    ?changed,
                    duration_ms = t0.elapsed().as_millis(),
                    duration_micros = t0.elapsed().as_micros(),
                    "LuaModulePort refresh completed successfully"
                );
            }
            Err(e) => {
                tracing::error!(?changed, err = ?e, "LuaModulePort refresh failed");
            }
        }
    }

    fn render(&self, monitor: &MonitorId) -> crate::features::vdom::domain::VNode {
        let t0 = std::time::Instant::now();
        let render_result = {
            let lua = self
                .lua
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let globals = lua.globals();

            let lua_monitor = {
                let mut mon_info = None;
                if let Ok(Some(raw_monitors)) = globals.get::<Option<mlua::Table>>("_raw_monitors") {
                    for pair in raw_monitors.sequence_values::<mlua::Value>() {
                        if let Ok(mlua::Value::UserData(ud)) = pair
                            && let Ok(mon) = ud.borrow::<LuaMonitor>()
                            && mon.0.id() == monitor
                        {
                            mon_info = Some(mon.0.clone());
                            break;
                        }
                    }
                }
                LuaMonitor(mon_info.unwrap_or_else(|| ScriptMonitorInfo::from_id(monitor)))
            };

            let res = globals.get::<mlua::Function>("render").map(|render_fn| {
                render_fn
                    .call::<mlua::Value>(lua_monitor)
                    .map(|val| value_to_vnode(&lua, val))
            });
            drop(lua);
            res
        };

        match render_result {
            Ok(Ok(Ok(node))) => {
                tracing::debug!(
                    module = %self.name,
                    monitor = %monitor,
                    duration_ms = t0.elapsed().as_millis(),
                    duration_micros = t0.elapsed().as_micros(),
                    "Lua render completed successfully"
                );
                return node;
            }
            Ok(Ok(Err(e))) => {
                tracing::error!(
                    module = %self.name,
                    monitor = %monitor,
                    err = ?e,
                    "Failed to convert Lua return value to VNode"
                );
            }
            Ok(Err(e)) => {
                tracing::error!(
                    module = %self.name,
                    monitor = %monitor,
                    err = ?e,
                    "Lua render_fn execution failed"
                );
            }
            Err(_) => {}
        }

        crate::features::vdom::domain::VNode::new_flex(vec![], None, None, None, None, None)
    }

    fn call_function_with_args(
        &mut self,
        name: &crate::shared::primitives::FunctionName,
        args: &[&str],
    ) -> Result<(), ModuleInitError> {
        let res = {
            let lua = self
                .lua
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let globals = lua.globals();

            let call_res = globals.get::<mlua::Function>(name.as_str()).map(|func| {
                args.first()
                    .map_or_else(|| func.call::<()>(()), |arg0| func.call::<()>(*arg0))
            });
            drop(globals);
            drop(lua);
            call_res
        };

        res.unwrap_or(Ok(())).map_err(|e| {
            ModuleInitError::ScriptError(format!("Failed to call function '{name}': {e}"))
        })
    }
}
