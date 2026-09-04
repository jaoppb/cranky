#![allow(unsafe_code)]

use crate::features::module_runtime::ports::AnyModulePort;
use crate::shared::config::domain::ModuleConfig;
use crate::shared::events::signals::{SignalHub, SignalKind};
use crate::shared::primitives::{MonitorId, ScriptMonitorInfo};
use crate::shared::scripting::ports::ModuleError;
use rhai::{AST, Dynamic, Engine, Scope};
use std::sync::Mutex;

fn get_hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .unwrap_or_else(|_| {
            std::fs::read_to_string("/etc/hostname")
                .map_or_else(|_| "unknown".to_string(), |s| s.trim().to_string())
        })
}

fn monitor_info_to_rhai_map(info: &ScriptMonitorInfo) -> rhai::Map {
    let mut map = rhai::Map::new();
    map.insert("id".into(), Dynamic::from(info.id().as_str().to_string()));
    map.insert("name".into(), Dynamic::from(info.name().to_string()));
    map.insert("width".into(), Dynamic::from(i64::from(info.size().width())));
    map.insert("height".into(), Dynamic::from(i64::from(info.size().height())));
    map.insert(
        "scale".into(),
        Dynamic::from(f64::from(info.scale().value())),
    );
    map.insert("is_focused".into(), Dynamic::from(info.is_focused()));
    map.insert(
        "active_workspace_id".into(),
        info.active_workspace_id()
            .map_or(Dynamic::UNIT, |w| Dynamic::from(i64::from(w))),
    );
    map.insert(
        "special_workspace_id".into(),
        info.special_workspace_id()
            .map_or(Dynamic::UNIT, |w| Dynamic::from(i64::from(w))),
    );
    map
}

macro_rules! register_container_elements {
    ($engine:expr, $($name:literal),*) => {
        $(
            $engine.register_fn($name, |_target: rhai::Map, children: rhai::Array| -> rhai::Map {
                let mut m = rhai::Map::new();
                m.insert("type".into(), Dynamic::from($name));
                m.insert("children".into(), Dynamic::from(children));
                m
            });
            $engine.register_fn($name, |_target: rhai::Map, mut props: rhai::Map| -> rhai::Map {
                props.insert("type".into(), Dynamic::from($name));
                props
            });
            $engine.register_fn($name, |_target: rhai::Map| -> rhai::Map {
                let mut m = rhai::Map::new();
                m.insert("type".into(), Dynamic::from($name));
                m.insert("children".into(), Dynamic::from(rhai::Array::new()));
                m
            });
        )*
    };
}

macro_rules! register_text_elements {
    ($engine:expr, $($t:ty),*) => {
        $engine.register_fn("text", |_target: rhai::Map, mut props: rhai::Map| -> rhai::Map {
            props.insert("type".into(), Dynamic::from("text"));
            props
        });
        $(
            $engine.register_fn("text", |_target: rhai::Map, val: $t| -> rhai::Map {
                let mut m = rhai::Map::new();
                m.insert("type".into(), Dynamic::from("text"));
                m.insert("text".into(), Dynamic::from(val.to_string()));
                m
            });
            $engine.register_fn("text", |_target: rhai::Map, val: $t, class: String| -> rhai::Map {
                let mut m = rhai::Map::new();
                m.insert("type".into(), Dynamic::from("text"));
                m.insert("text".into(), Dynamic::from(val.to_string()));
                m.insert("class".into(), Dynamic::from(class));
                m
            });
        )*
    };
}

macro_rules! register_script_call_aliases {
    ($engine:expr, $($name:literal),*) => {
        $(
            $engine.register_fn($name, |_target: rhai::Map, func_name: String| -> rhai::Map {
                let mut m = rhai::Map::new();
                m.insert("ScriptCall".into(), Dynamic::from(func_name));
                m
            });
        )*
    };
}

macro_rules! register_rhai_log_methods {
    ($engine:expr, $(($name:literal, $level:ident)),*) => {
        $(
            $engine.register_fn($name, |_target: rhai::Map, msg: String| {
                tracing::$level!("{msg}");
            });
        )*
    };
}

fn register_rhai_flex_and_grid(engine: &mut Engine) {
    register_container_elements!(engine, "flex", "grid");
}

fn register_rhai_text_and_progress(engine: &mut Engine) {
    register_text_elements!(engine, String, i64, f64);

    // progress
    engine.register_fn("progress", |_target: rhai::Map, mut props: rhai::Map| -> rhai::Map {
        props.insert("type".into(), Dynamic::from("progress"));
        props
    });
    engine.register_fn("progress", |_target: rhai::Map, val: f64| -> rhai::Map {
        let mut m = rhai::Map::new();
        m.insert("type".into(), Dynamic::from("progress"));
        m.insert("value".into(), Dynamic::from(val));
        m
    });
    engine.register_fn("progress", |_target: rhai::Map, val: f64, class: String| -> rhai::Map {
        let mut m = rhai::Map::new();
        m.insert("type".into(), Dynamic::from("progress"));
        m.insert("value".into(), Dynamic::from(val));
        m.insert("class".into(), Dynamic::from(class));
        m
    });
    engine.register_fn(
        "progress",
        |_target: rhai::Map, val: f64, orientation: String, class: String| -> rhai::Map {
            let mut m = rhai::Map::new();
            m.insert("type".into(), Dynamic::from("progress"));
            m.insert("value".into(), Dynamic::from(val));
            m.insert("orientation".into(), Dynamic::from(orientation));
            m.insert("class".into(), Dynamic::from(class));
            m
        },
    );
    engine.register_fn("progress", |_target: rhai::Map, val: i64| -> rhai::Map {
        let mut m = rhai::Map::new();
        m.insert("type".into(), Dynamic::from("progress"));
        #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
        m.insert("value".into(), Dynamic::from(val as f64));
        m
    });
    engine.register_fn("progress", |_target: rhai::Map, val: i64, class: String| -> rhai::Map {
        let mut m = rhai::Map::new();
        m.insert("type".into(), Dynamic::from("progress"));
        #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
        m.insert("value".into(), Dynamic::from(val as f64));
        m.insert("class".into(), Dynamic::from(class));
        m
    });
    engine.register_fn(
        "progress",
        |_target: rhai::Map, val: i64, orientation: String, class: String| -> rhai::Map {
            let mut m = rhai::Map::new();
            m.insert("type".into(), Dynamic::from("progress"));
            #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
            m.insert("value".into(), Dynamic::from(val as f64));
            m.insert("orientation".into(), Dynamic::from(orientation));
            m.insert("class".into(), Dynamic::from(class));
            m
        },
    );
}

fn register_rhai_rect_image_module(engine: &mut Engine) {
    // rect
    engine.register_fn("rect", |_target: rhai::Map, mut props: rhai::Map| -> rhai::Map {
        props.insert("type".into(), Dynamic::from("rect"));
        props
    });
    engine.register_fn("rect", |_target: rhai::Map, class: String| -> rhai::Map {
        let mut m = rhai::Map::new();
        m.insert("type".into(), Dynamic::from("rect"));
        m.insert("class".into(), Dynamic::from(class));
        m
    });
    engine.register_fn("rect", |_target: rhai::Map| -> rhai::Map {
        let mut m = rhai::Map::new();
        m.insert("type".into(), Dynamic::from("rect"));
        m
    });

    // image
    engine.register_fn("image", |_target: rhai::Map, mut props: rhai::Map| -> rhai::Map {
        props.insert("type".into(), Dynamic::from("image"));
        props
    });

    // module / widget / load_module
    for fn_name in ["module", "widget", "load_module", "mod_element"] {
        engine.register_fn(fn_name, |_target: rhai::Map, mut props: rhai::Map| -> rhai::Map {
            props.insert("type".into(), Dynamic::from("module"));
            props
        });
        engine.register_fn(fn_name, |_target: rhai::Map, name: String| -> rhai::Map {
            let mut m = rhai::Map::new();
            m.insert("type".into(), Dynamic::from("module"));
            m.insert("name".into(), Dynamic::from(name));
            m
        });
        engine.register_fn(
            fn_name,
            |_target: rhai::Map, name: String, class: String| -> rhai::Map {
                let mut m = rhai::Map::new();
                m.insert("type".into(), Dynamic::from("module"));
                m.insert("name".into(), Dynamic::from(name));
                m.insert("class".into(), Dynamic::from(class));
                m
            },
        );
    }
}

fn register_rhai_ui_elements(engine: &mut Engine) {
    register_rhai_flex_and_grid(engine);
    register_rhai_text_and_progress(engine);
    register_rhai_rect_image_module(engine);
}

fn register_rhai_ui_actions(engine: &mut Engine) {
    engine.register_fn("exec", |target: rhai::Map, cmd: String| -> Dynamic {
        let is_action = target
            .get("_ns")
            .and_then(|v| v.clone().try_cast::<String>())
            .as_deref()
            == Some("ui.action");
        if is_action {
            let mut m = rhai::Map::new();
            m.insert("Exec".into(), Dynamic::from(cmd));
            return Dynamic::from(m);
        }
        let _ = std::process::Command::new("sh").arg("-c").arg(&cmd).spawn();
        Dynamic::UNIT
    });

    register_script_call_aliases!(engine, "script_call", "call_script", "func", "call");

    engine.register_fn(
        "systray",
        |_target: rhai::Map, id: String, action: String| -> rhai::Map {
            let mut inner = rhai::Map::new();
            inner.insert("id".into(), Dynamic::from(id));
            inner.insert("action".into(), Dynamic::from(action));
            let mut m = rhai::Map::new();
            m.insert("SystrayAction".into(), Dynamic::from(inner));
            m
        },
    );

    engine.register_fn("systray", |_target: rhai::Map, id: String| -> rhai::Map {
        let mut inner = rhai::Map::new();
        inner.insert("id".into(), Dynamic::from(id));
        inner.insert("action".into(), Dynamic::from("Primary".to_string()));
        let mut m = rhai::Map::new();
        m.insert("SystrayAction".into(), Dynamic::from(inner));
        m
    });

    engine.register_fn(
        "systray",
        |_target: rhai::Map, id: i64, action: String| -> rhai::Map {
            let mut inner = rhai::Map::new();
            inner.insert("id".into(), Dynamic::from(id.to_string()));
            inner.insert("action".into(), Dynamic::from(action));
            let mut m = rhai::Map::new();
            m.insert("SystrayAction".into(), Dynamic::from(inner));
            m
        },
    );

    engine.register_fn("systray", |_target: rhai::Map, id: i64| -> rhai::Map {
        let mut inner = rhai::Map::new();
        inner.insert("id".into(), Dynamic::from(id.to_string()));
        inner.insert("action".into(), Dynamic::from("Primary".to_string()));
        let mut m = rhai::Map::new();
        m.insert("SystrayAction".into(), Dynamic::from(inner));
        m
    });
}

fn register_rhai_sys(engine: &mut Engine) {
    // env & hostname
    engine.register_fn("env", |_target: rhai::Map, key: String| -> Dynamic {
        std::env::var(key).map_or(Dynamic::UNIT, Dynamic::from)
    });
    engine.register_fn("hostname", |_target: rhai::Map| -> String {
        get_hostname()
    });

    // log
    register_rhai_log_methods!(
        engine,
        ("info", info),
        ("warn", warn),
        ("error", error),
        ("debug", debug)
    );

    // monitors
    engine.register_fn("all", |target: rhai::Map| -> rhai::Array {
        target
            .get("_list")
            .and_then(|v| v.clone().try_cast::<rhai::Array>())
            .unwrap_or_default()
    });
    engine.register_fn("current", |_target: rhai::Map, cur: rhai::Map| -> rhai::Map {
        cur
    });
    engine.register_fn("focused", |target: rhai::Map| -> Dynamic {
        let list = target
            .get("_list")
            .and_then(|v| v.clone().try_cast::<rhai::Array>())
            .unwrap_or_default();
        for item in list {
            let Some(map) = item.clone().try_cast::<rhai::Map>() else {
                continue;
            };
            let is_focused = map
                .get("is_focused")
                .and_then(|v| v.clone().try_cast::<bool>())
                .unwrap_or(false);
            if is_focused {
                return Dynamic::from(map);
            }
        }
        Dynamic::UNIT
    });
    engine.register_fn("by_name", |target: rhai::Map, name: String| -> Dynamic {
        let list = target
            .get("_list")
            .and_then(|v| v.clone().try_cast::<rhai::Array>())
            .unwrap_or_default();
        for item in list {
            let Some(map) = item.clone().try_cast::<rhai::Map>() else {
                continue;
            };
            let item_name = map
                .get("name")
                .and_then(|v| v.clone().try_cast::<String>());
            if item_name.as_deref() == Some(name.as_str()) {
                return Dynamic::from(map);
            }
        }
        Dynamic::UNIT
    });

    // monitor to_string
    engine.register_fn("to_string", |m: rhai::Map| -> String {
        m.get("id")
            .and_then(|v| v.clone().try_cast::<String>())
            .unwrap_or_default()
    });
}

pub fn register_rhai_cranky_api(engine: &mut Engine) {
    register_rhai_ui_elements(engine);
    register_rhai_ui_actions(engine);
    register_rhai_sys(engine);

    // Standalone exec for backwards compatibility
    engine.register_fn("exec", |cmd: String| {
        let _ = std::process::Command::new("sh").arg("-c").arg(&cmd).spawn();
    });

    // Script assertions
    engine.register_fn("assert", |cond: bool| -> Result<(), Box<rhai::EvalAltResult>> {
        if !cond {
            return Err(rhai::EvalAltResult::ErrorRuntime(
                Dynamic::from("Assertion failed in Rhai script"),
                rhai::Position::NONE,
            )
            .into());
        }
        Ok(())
    });
    engine.register_fn(
        "assert",
        |cond: bool, msg: String| -> Result<(), Box<rhai::EvalAltResult>> {
            if !cond {
                return Err(rhai::EvalAltResult::ErrorRuntime(
                    Dynamic::from(format!("Assertion failed in Rhai script: {msg}")),
                    rhai::Position::NONE,
                )
                .into());
            }
            Ok(())
        },
    );
}

fn create_initial_scope() -> Scope<'static> {
    let mut scope = Scope::new();

    // ui
    let mut ui_map = rhai::Map::new();
    let ui_element_map = rhai::Map::new();
    let mut ui_action_map = rhai::Map::new();
    ui_action_map.insert("_ns".into(), Dynamic::from("ui.action"));
    ui_map.insert("element".into(), Dynamic::from(ui_element_map));
    ui_map.insert("action".into(), Dynamic::from(ui_action_map));

    // signals
    let signals_map = rhai::Map::new();

    // config
    let config_map = rhai::Map::new();

    // sys
    let mut sys_map = rhai::Map::new();
    sys_map.insert("_ns".into(), Dynamic::from("sys"));
    let sys_log_map = rhai::Map::new();
    let sys_monitors_map = rhai::Map::new();
    sys_map.insert("log".into(), Dynamic::from(sys_log_map));
    sys_map.insert("monitors".into(), Dynamic::from(sys_monitors_map));

    // cranky
    let mut cranky_map = rhai::Map::new();
    cranky_map.insert("ui".into(), Dynamic::from(ui_map.clone()));
    cranky_map.insert("signals".into(), Dynamic::from(signals_map.clone()));
    cranky_map.insert("config".into(), Dynamic::from(config_map.clone()));
    cranky_map.insert("sys".into(), Dynamic::from(sys_map.clone()));

    scope.push("cranky", cranky_map);
    scope.push("ui", ui_map);
    scope.push("signals", signals_map);
    scope.push("config", config_map);
    scope.push("sys", sys_map);

    scope
}

pub struct RhaiModule {
    engine: Mutex<Engine>,
    scope: Mutex<Scope<'static>>,
    ast: AST,
    name: String,
    cached_subs: Vec<SignalKind>,
    cached_dbus_subs: Vec<crate::shared::dbus::domain::DBusSubscription>,
    cached_styles: Vec<crate::features::styling::domain::StyleSheetName>,
    cached_monitors: Vec<ScriptMonitorInfo>,
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
            "clock" => Some(include_str!("../../../../assets/widgets/clock.rhai")),
            "calendar" => Some(include_str!("../../../../assets/widgets/calendar.rhai")),
            "workspace" => Some(include_str!("../../../../assets/widgets/workspace.rhai")),
            "systray" => Some(include_str!("../../../../assets/widgets/systray.rhai")),
            "metrics" => Some(include_str!("../../../../assets/widgets/metrics.rhai")),
            "mpris" => Some(include_str!("../../../../assets/widgets/mpris.rhai")),
            "bar" => Some(include_str!("../../../../assets/widgets/bar.rhai")),
            _ => None,
        }?;
        Self::new(name.to_string(), source).ok()
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

        // Expose module config options
        let options_json = serde_json::to_string(config.options())
            .map_err(|e| ModuleInitError::ConfigError(e.to_string()))?;
        let options_rhai: rhai::Map = engine
            .parse_json(&options_json, true)
            .map_err(|e| ModuleInitError::ScriptError(e.to_string()))?;

        // Update config map
        let mut config_map = scope.get_value::<rhai::Map>("config").unwrap_or_default();
        config_map.insert("module".into(), Dynamic::from(options_rhai.clone()));
        config_map.insert("options".into(), Dynamic::from(options_rhai));
        config_map.insert("root".into(), Dynamic::from(root_map));

        scope.set_value("config", config_map.clone());

        // Update cranky.config
        if let Some(mut cranky_map) = scope.get_value::<rhai::Map>("cranky") {
            cranky_map.insert("config".into(), Dynamic::from(config_map));
            scope.set_value("cranky", cranky_map);
        }

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

        let mut signals_map = scope.get_value::<rhai::Map>("signals").unwrap_or_default();

        if changed.contains(&SignalKind::Time) {
            let time = *hub.time_rx().borrow();
            signals_map.insert("time".into(), Dynamic::from(time.to_rfc3339()));
        }

        if changed.contains(&SignalKind::Hyprland) {
            let hypr = hub.hyprland_rx().borrow().clone();
            if let Ok(hypr_json) = serde_json::to_string(&hypr)
                && let Ok(hypr_rhai) = engine.parse_json(&hypr_json, true)
            {
                signals_map.insert("hyprland".into(), Dynamic::from(hypr_rhai));
            }
        }

        if changed.contains(&SignalKind::Systray) {
            let systray = hub.systray_rx().borrow().clone();
            let items = systray.items().values().collect::<Vec<_>>();
            if let Ok(systray_json) = serde_json::to_string(&items)
                && let Ok(systray_rhai) = engine.parse_json(&systray_json, true)
            {
                signals_map.insert("systray".into(), Dynamic::from(systray_rhai));
            }
        }

        if changed.contains(&SignalKind::Metrics) {
            let metrics = hub.metrics_rx().borrow().clone();
            if let Ok(metrics_json) = serde_json::to_string(&metrics)
                && let Ok(metrics_rhai) = engine.parse_json(&metrics_json, true)
            {
                signals_map.insert("metrics".into(), Dynamic::from(metrics_rhai));
            }
        }

        if changed.contains(&SignalKind::Mpris) {
            let mpris = hub.mpris_rx().borrow().clone();
            if let Ok(mpris_json) = serde_json::to_string(&mpris)
                && let Ok(mpris_rhai) = engine.parse_json(&mpris_json, true)
            {
                signals_map.insert("mpris".into(), Dynamic::from(mpris_rhai));
            }
        }

        let mut dbus_handled = false;
        for signal in changed {
            if matches!(signal, SignalKind::DBus) && !dbus_handled {
                let dbus_state = hub.dbus_rx().borrow().clone();
                if let Ok(dbus_json) = serde_json::to_string(&dbus_state.properties())
                    && let Ok(dbus_rhai) = engine.parse_json(&dbus_json, true)
                {
                    signals_map.insert("dbus".into(), Dynamic::from(dbus_rhai));
                }
                dbus_handled = true;
            }
        }

        scope.set_value("signals", signals_map.clone());

        // Update sys.monitors list in scope
        let monitor_infos = hub.get_monitor_infos();
        self.cached_monitors.clone_from(&monitor_infos);
        let monitors_arr: rhai::Array = monitor_infos
            .iter()
            .map(monitor_info_to_rhai_map)
            .map(Dynamic::from)
            .collect();

        let mut sys_map = scope.get_value::<rhai::Map>("sys").unwrap_or_default();
        let mut monitors_map = sys_map
            .get("monitors")
            .and_then(|v| v.clone().try_cast::<rhai::Map>())
            .unwrap_or_default();
        monitors_map.insert("_list".into(), Dynamic::from(monitors_arr));
        sys_map.insert("monitors".into(), Dynamic::from(monitors_map));
        scope.set_value("sys", sys_map.clone());

        // Update cranky map
        if let Some(mut cranky_map) = scope.get_value::<rhai::Map>("cranky") {
            cranky_map.insert("signals".into(), Dynamic::from(signals_map));
            cranky_map.insert("sys".into(), Dynamic::from(sys_map));
            scope.set_value("cranky", cranky_map);
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
                if let rhai::EvalAltResult::ErrorFunctionNotFound(f, ..) = &*e
                    && f == name.as_str()
                {
                    return Ok(());
                }
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
    use crate::features::vdom::domain::UiAction;
    use crate::shared::events::core::PointerButton;
    use crate::shared::events::signals::SignalKind;

    #[test]
    fn test_rhai_cranky_namespaces_and_aliases() {
        let source = r#"
            fn init() {
                // cranky namespaces
                assert(cranky != ());
                assert(cranky.ui != ());
                assert(cranky.signals != ());
                assert(cranky.config != ());
                assert(cranky.sys != ());

                // aliases
                assert(ui != ());
                assert(signals != ());
                assert(config != ());
                assert(sys != ());
            }
            fn refresh() {}
            fn render(monitor) {
                let txt = ui.text("Namespace Test", "title");
                let btn = ui.element.text("Element Alias");
                return ui.flex([txt, btn]);
            }
        "#;
        let mut module = RhaiModule::new("test_ns".into(), source).unwrap();
        let mod_config = crate::shared::config::domain::ModuleConfig::new(
            "test_ns".into(),
            true,
            crate::shared::config::domain::EngineSelection::Auto,
            crate::shared::primitives::ModuleOptions::default(),
        );
        let config = crate::shared::config::domain::Config::default();
        assert!(module.init(&mod_config, &config).is_ok());
        let node = module.render(&MonitorId::new("DP-1"));
        assert_eq!(node.tag(), crate::features::vdom::domain::NodeTag::Flex);
        assert_eq!(node.children().len(), 2);
    }

    #[test]
    fn test_rhai_ui_actions() {
        let source = r#"
            fn init() {}
            fn refresh() {}
            fn render(monitor) {
                let a1 = ui.action.exec("echo test");
                let a2 = ui.action.script_call("toggle_popup");
                let a3 = ui.action.systray(42, "Activate");

                return ui.flex(#{
                    children: [
                        ui.text(#{ text: "Click", on_click: a1 }),
                        ui.text(#{ text: "Call", on_click: a2 }),
                        ui.text(#{ text: "Systray", on_click: a3 })
                    ]
                });
            }
        "#;
        let module = RhaiModule::new("test_actions".into(), source).unwrap();
        let node = module.render(&MonitorId::new("DP-1"));
        assert_eq!(node.children().len(), 3);

        let click1 = node.children()[0].on_click().unwrap();
        assert_eq!(
            click1.get(&PointerButton::Left),
            Some(&UiAction::Exec("echo test".into()))
        );

        let click2 = node.children()[1].on_click().unwrap();
        assert_eq!(
            click2.get(&PointerButton::Left),
            Some(&UiAction::ScriptCall(
                crate::shared::primitives::FunctionName::new("toggle_popup")
            ))
        );

        let click3 = node.children()[2].on_click().unwrap();
        assert_eq!(
            click3.get(&PointerButton::Left),
            Some(&UiAction::SystrayAction {
                id: crate::features::systray::domain::SystrayId::new("42"),
                action: crate::features::systray::domain::SystrayActionName::parse_str("Activate"),
                pos: None
            })
        );
    }

    #[test]
    fn test_rhai_rich_monitor_and_sys() {
        let source = r#"
            fn init() {}
            fn refresh() {}
            fn render(monitor) {
                // Assert monitor properties
                assert(monitor.id == "DP-1");
                assert(monitor.name == "DP-1");
                assert(monitor.width == 1920);
                assert(monitor.height == 1080);
                assert(monitor.scale == 1.5);
                assert(monitor.is_focused == true);
                assert(monitor.active_workspace_id == 1);
                assert(monitor.special_workspace_id == 2);
                assert(to_string(monitor) == "DP-1");

                // Assert sys
                assert(sys.hostname() != "");
                sys.log.info("Rendering on monitor " + monitor.name);

                let cur = sys.monitors.current(monitor);
                assert(cur.id == monitor.id);

                return ui.text("Mon: " + monitor.name);
            }
        "#;
        let mut module = RhaiModule::new("test_sys".into(), source).unwrap();
        let test_mon = ScriptMonitorInfo::new(
            MonitorId::new("DP-1"),
            "DP-1".to_string(),
            crate::shared::primitives::geometry::Size::new(1920, 1080),
            crate::shared::primitives::geometry::Scale::new(1.5),
            true,
            Some(1),
            Some(2),
        );
        module.cached_monitors = vec![test_mon];

        let node = module.render(&MonitorId::new("DP-1"));
        assert_eq!(node.tag(), crate::features::vdom::domain::NodeTag::Text);
    }

    #[test]
    fn test_rhai_module_lifecycle() {
        let source = "
            fn subscriptions() { return [\"time\", \"hyprland\", \"metrics\"]; }
            fn refresh() {}
            fn render(monitor) {
                return ui.flex([]);
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
    fn test_rhai_render_grid() {
        let source = r#"
            fn init() {}
            fn refresh() {}
            fn render(monitor) {
                return ui.grid([
                    ui.text("Grid item 1"),
                    ui.text("Grid item 2")
                ]);
            }
        "#;
        let module = RhaiModule::new("test_grid".into(), source).unwrap();
        let render_node = module.render(&MonitorId::new("DP-1"));
        assert_eq!(
            render_node.tag(),
            crate::features::vdom::domain::NodeTag::Grid
        );
        assert_eq!(render_node.children().len(), 2);
    }

    #[test]
    fn test_rhai_call_function_optional_not_found() {
        use crate::features::module_runtime::ports::AnyModulePort;
        use crate::shared::primitives::FunctionName;

        let source = r#"
            fn init() {}
            fn refresh() {}
            fn render(monitor) {
                return ui.text("hi");
            }
        "#;
        let mut module = RhaiModule::new("test_optional".into(), source).unwrap();
        let res = module.call_function(&FunctionName::new("non_existent_fn"));
        assert!(
            res.is_ok(),
            "Calling undefined function should return Ok(())"
        );
    }

    #[test]
    fn test_rhai_all_builtins() {
        for (name, source) in [
            ("clock", include_str!("../../../../assets/widgets/clock.rhai")),
            ("calendar", include_str!("../../../../assets/widgets/calendar.rhai")),
            ("workspace", include_str!("../../../../assets/widgets/workspace.rhai")),
            ("systray", include_str!("../../../../assets/widgets/systray.rhai")),
            ("metrics", include_str!("../../../../assets/widgets/metrics.rhai")),
            ("mpris", include_str!("../../../../assets/widgets/mpris.rhai")),
            ("bar", include_str!("../../../../assets/widgets/bar.rhai")),
        ] {
            let res = RhaiModule::new(name.into(), source);
            if let Err(e) = &res {
                eprintln!("ERROR for {name}: {e:?}");
            }
            assert!(res.is_ok(), "Failed to load builtin rhai module {name}: {:?}", res.err());
        }
    }
}

