use super::api::register_rhai_cranky_api;
use super::helpers::create_initial_scope;
use crate::shared::events::signals::SignalKind;
use crate::shared::primitives::ScriptMonitorInfo;
use crate::shared::scripting::ports::ModuleError;
use rhai::{AST, Engine, Scope};
use std::sync::Mutex;

pub struct RhaiModule {
    pub(crate) engine: Mutex<Engine>,
    pub(crate) scope: Mutex<Scope<'static>>,
    pub(crate) ast: AST,
    pub(crate) name: String,
    pub(crate) cached_subs: Vec<SignalKind>,
    pub(crate) cached_dbus_subs: Vec<crate::shared::dbus::domain::DBusSubscription>,
    pub(crate) cached_styles: Vec<crate::features::styling::domain::StyleSheetName>,
    pub(crate) cached_monitors: Vec<ScriptMonitorInfo>,
}

impl RhaiModule {
    /// # Errors
    ///
    /// Returns `ModuleError::Internal` if compiling or evaluating the script fails.
    pub fn new(name: String, source: &str) -> Result<Self, ModuleError> {
        let mut engine = Engine::new();
        engine.set_max_expr_depths(0, 0);

        register_rhai_cranky_api(&mut engine);

        let ast = engine.compile(source).map_err(|e| ModuleError::Internal {
            message: format!("Failed to compile Rhai script {name}: {e}"),
        })?;

        let mut scope = create_initial_scope();

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
            cached_monitors: Vec::new(),
        })
    }

    #[cfg(test)]
    #[must_use]
    pub fn built_in(name: &str) -> Option<Self> {
        let source = match name {
            "clock" => Some(include_str!("../../../../../assets/widgets/clock.rhai")),
            "calendar" => Some(include_str!("../../../../../assets/widgets/calendar.rhai")),
            "workspace" => Some(include_str!("../../../../../assets/widgets/workspace.rhai")),
            "systray" => Some(include_str!("../../../../../assets/widgets/systray.rhai")),
            "metrics" => Some(include_str!("../../../../../assets/widgets/metrics.rhai")),
            "mpris" => Some(include_str!("../../../../../assets/widgets/mpris.rhai")),
            "bar" => Some(include_str!("../../../../../assets/widgets/bar.rhai")),
            _ => None,
        }?;
        Self::new(name.to_string(), source).ok()
    }
}
