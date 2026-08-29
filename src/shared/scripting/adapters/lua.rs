use crate::app::commands::AppCommand;
use crate::features::module_runtime::ports::{AnyModulePort, ModuleInitError};
use crate::features::styling::domain::{ClassNameList, ElementId, Orientation, ProgressValue};
use crate::features::vdom::domain::{TextContent, VNode};
use crate::shared::config::domain::ModuleConfig;
use crate::shared::dbus::domain::{BusType, DBusSubscription};
use crate::shared::events::signals::{SignalHub, SignalKind};
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::{
    BinaryData, ModuleInstanceId, ModuleName, ModuleOptions, MonitorId,
};
use mlua::{Function, Lua, LuaSerdeExt, UserData, UserDataMethods};
use std::sync::Mutex;

#[derive(Clone)]
pub struct LuaVNode(pub VNode);

impl UserData for LuaVNode {}

#[derive(Clone)]
pub struct LuaMonitor(pub MonitorId);

impl UserData for LuaMonitor {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("id", |_, this, ()| Ok(this.0.as_str().to_string()));
    }
}

#[cfg(test)]
pub struct LuaScriptLoader;

#[cfg(test)]
impl LuaScriptLoader {
    #[must_use]
    pub fn load_built_in(name: &str) -> Option<String> {
        match name {
            "hour" => Some(include_str!("../../../../assets/widgets/hour.lua").to_string()),
            "workspace" => {
                Some(include_str!("../../../../assets/widgets/workspace.lua").to_string())
            }
            "systray" => Some(include_str!("../../../../assets/widgets/systray.lua").to_string()),
            "metrics" => Some(include_str!("../../../../assets/widgets/metrics.lua").to_string()),
            _ => None,
        }
    }
}

pub struct LuaStateSynchronizer;

impl LuaStateSynchronizer {
    /// # Errors
    ///
    /// Returns `mlua::Error` if setting globals fails.
    pub fn sync(lua: &Lua, hub: &SignalHub, changed: &[SignalKind]) -> mlua::Result<()> {
        let globals = lua.globals();
        let mut dbus_handled = false;

        for signal in changed {
            match signal {
                SignalKind::Time => {
                    let time = *hub.time_rx().borrow();
                    globals.set("current_time", time.to_rfc3339())?;
                }
                SignalKind::Hyprland => {
                    let hypr = hub.hyprland_rx().borrow().clone();
                    if let Ok(val) = lua.to_value(&hypr) {
                        globals.set("hyprland", val)?;
                    }
                }
                SignalKind::DBus if !dbus_handled => {
                    let dbus_state = hub.dbus_rx().borrow().clone();
                    if let Ok(val) = lua.to_value(&dbus_state.properties()) {
                        globals.set("dbus", val)?;
                    }
                    dbus_handled = true;
                }
                SignalKind::Systray => {
                    let t0 = std::time::Instant::now();
                    let systray = hub.systray_rx().borrow().clone();
                    let items = systray.items().values().collect::<Vec<_>>();
                    let item_count = items.len();
                    tracing::debug!(item_count, "Serializing systray to Lua");
                    match lua.to_value(&items) {
                        Ok(val) => {
                            globals.set("systray", val)?;
                            tracing::debug!(
                                item_count,
                                duration_ms = t0.elapsed().as_millis(),
                                duration_micros = t0.elapsed().as_micros(),
                                "Successfully serialized systray to Lua"
                            );
                        }
                        Err(e) => tracing::error!(err = ?e, "Failed to serialize systray to Lua"),
                    }
                }
                SignalKind::Metrics => {
                    let metrics = hub.metrics_rx().borrow().clone();
                    if let Ok(val) = lua.to_value(&metrics) {
                        globals.set("metrics", val)?;
                    }
                }
                SignalKind::Mpris => {
                    let mpris = hub.mpris_rx().borrow().clone();
                    if let Ok(val) = lua.to_value(&mpris) {
                        let _ = globals.set("mpris", val);
                    }
                }
                SignalKind::DBus => {}
            }
        }

        if let Ok(refresh_fn) = globals.get::<Function>("refresh") {
            let t0 = std::time::Instant::now();
            match refresh_fn.call::<()>(()) {
                Ok(()) => {
                    tracing::debug!(
                        duration_ms = t0.elapsed().as_millis(),
                        duration_micros = t0.elapsed().as_micros(),
                        "Lua refresh function called successfully"
                    );
                }
                Err(e) => {
                    tracing::error!(err = ?e, "Lua refresh function failed");
                }
            }
        }

        Ok(())
    }
}

fn parse_app_command(lua: &Lua, val: Option<mlua::Value>) -> Option<AppCommand> {
    match val {
        Some(mlua::Value::Table(t)) => {
            let cmd: Result<AppCommand, _> = lua.from_value(mlua::Value::Table(t));
            cmd.ok()
        }
        _ => None,
    }
}

fn parse_pixel_size(table: &mlua::Table) -> mlua::Result<Size> {
    let Some(size_table) = table.get::<Option<mlua::Table>>("pixel_size")? else {
        return Ok(Size::new(0, 0));
    };
    let w = size_table.get::<Option<u32>>("width")?.unwrap_or(0);
    let h = size_table.get::<Option<u32>>("height")?.unwrap_or(0);
    Ok(Size::new(w, h))
}

fn parse_image_data(lua: &Lua, table: &mlua::Table) -> BinaryData {
    match table.get::<mlua::Value>("data") {
        Ok(mlua::Value::String(s)) => BinaryData::new(s.as_bytes().to_vec()),
        Ok(val) => lua
            .from_value(val)
            .unwrap_or_else(|_| BinaryData::new(vec![])),
        Err(_) => BinaryData::new(vec![]),
    }
}

fn parse_image_data_and_size(lua: &Lua, table: &mlua::Table) -> mlua::Result<(BinaryData, Size)> {
    let size = parse_pixel_size(table)?;
    let data = parse_image_data(lua, table);
    Ok((data, size))
}

fn parse_text_content(table: &mlua::Table) -> String {
    match table.get::<mlua::Value>("text") {
        Ok(mlua::Value::String(s)) => s.to_str().map_or_else(|_| String::new(), |b| b.to_string()),
        Ok(mlua::Value::Integer(i)) => i.to_string(),
        Ok(mlua::Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

fn parse_module_options(lua: &Lua, table: &mlua::Table) -> mlua::Result<ModuleOptions> {
    Ok(table
        .get::<Option<mlua::Value>>("options")?
        .and_then(|opts_val| lua.from_value(opts_val).ok())
        .unwrap_or_default())
}

type CommonProps = (
    Option<ClassNameList>,
    Option<ElementId>,
    Option<AppCommand>,
    Option<AppCommand>,
    Option<Box<VNode>>,
);

fn parse_common_props(lua: &Lua, table: &mlua::Table) -> mlua::Result<CommonProps> {
    let class = table
        .get::<Option<String>>("class")?
        .and_then(|s| ClassNameList::parse(&s).ok());
    let id = table
        .get::<Option<String>>("id")?
        .and_then(|s| ElementId::new(s).ok());
    let on_click = parse_app_command(lua, table.get::<Option<mlua::Value>>("on_click")?);
    let on_hover = parse_app_command(lua, table.get::<Option<mlua::Value>>("on_hover")?);
    let tooltip = table
        .get::<Option<mlua::Value>>("tooltip")?
        .map(|tt| value_to_vnode(lua, tt).map(Box::new))
        .transpose()?;
    Ok((class, id, on_click, on_hover, tooltip))
}

/// Converts a Lua value (either `LuaVNode` `UserData` or a table) into a `VNode`.
///
/// # Errors
///
/// Returns `mlua::Error` if value conversion fails.
pub fn value_to_vnode(lua: &Lua, val: mlua::Value) -> mlua::Result<VNode> {
    match val {
        mlua::Value::UserData(ud) => {
            if let Ok(lua_vnode) = ud.borrow::<LuaVNode>() {
                return Ok(lua_vnode.0.clone());
            }
            Err(mlua::Error::FromLuaConversionError {
                from: "UserData",
                to: "VNode".to_string(),
                message: Some("UserData is not a LuaVNode".to_string()),
            })
        }
        mlua::Value::Table(table) => {
            let typ: String = table.get::<Option<String>>("type")?.unwrap_or_default();
            let (class, id, on_click, on_hover, tooltip) = parse_common_props(lua, &table)?;

            match typ.as_str() {
                "flex" | "" => {
                    let mut children_vec = Vec::new();
                    if let Ok(children) = table.get::<mlua::Table>("children") {
                        for pair in children.sequence_values::<mlua::Value>() {
                            let child_val = pair?;
                            children_vec.push(value_to_vnode(lua, child_val)?);
                        }
                    }
                    Ok(VNode::new_flex(
                        children_vec,
                        class,
                        id,
                        on_click,
                        on_hover,
                        tooltip,
                    ))
                }
                "text" => {
                    let text_str = parse_text_content(&table);
                    Ok(VNode::new_text(
                        TextContent::new(text_str),
                        class,
                        id,
                        on_click,
                        on_hover,
                        tooltip,
                    ))
                }
                "progress" => {
                    let value_num = table.get::<Option<f32>>("value")?.unwrap_or(0.0);
                    let orientation_str = table
                        .get::<Option<String>>("orientation")?
                        .unwrap_or_default();
                    let orientation = match orientation_str.to_lowercase().as_str() {
                        "vertical" => Orientation::Vertical,
                        _ => Orientation::Horizontal,
                    };
                    Ok(VNode::new_progress(
                        ProgressValue::new(value_num).unwrap_or_default(),
                        orientation,
                        class,
                        id,
                        on_click,
                        on_hover,
                        tooltip,
                    ))
                }
                "rect" => Ok(VNode::new_rect(class, id, on_click, on_hover, tooltip)),
                "image" => {
                    let (data, pixel_size) = parse_image_data_and_size(lua, &table)?;
                    Ok(VNode::new_image(data, pixel_size, class, id, tooltip))
                }
                "module" => {
                    let name_str = table.get::<String>("name")?;
                    let instance_id_str = table.get::<Option<String>>("instance_id")?;
                    let options = parse_module_options(lua, &table)?;
                    Ok(VNode::new_module(
                        ModuleName::new(name_str),
                        instance_id_str.map(ModuleInstanceId::new),
                        options,
                        class,
                        id,
                        on_click,
                        on_hover,
                        tooltip,
                    ))
                }
                _ => lua.from_value::<VNode>(mlua::Value::Table(table)),
            }
        }
        other => lua.from_value::<VNode>(other),
    }
}

/// Registers the `vdom` DSL table in Lua globals.
///
/// # Errors
///
/// Returns `mlua::Error` if table creation or function registration fails.
#[allow(clippy::too_many_lines)]
pub fn register_vdom_dsl(lua: &Lua) -> mlua::Result<()> {
    let vdom = lua.create_table()?;

    vdom.set(
        "flex",
        lua.create_function(|lua, table: mlua::Table| {
            let mut children_vec = Vec::new();
            if let Ok(children) = table.get::<mlua::Table>("children") {
                for pair in children.sequence_values::<mlua::Value>() {
                    let val = pair?;
                    children_vec.push(value_to_vnode(lua, val)?);
                }
            }
            let (class, id, on_click, on_hover, tooltip) = parse_common_props(lua, &table)?;
            Ok(LuaVNode(VNode::new_flex(
                children_vec,
                class,
                id,
                on_click,
                on_hover,
                tooltip,
            )))
        })?,
    )?;

    vdom.set(
        "text",
        lua.create_function(|lua, table: mlua::Table| {
            let text_str = parse_text_content(&table);
            let (class, id, on_click, on_hover, tooltip) = parse_common_props(lua, &table)?;
            Ok(LuaVNode(VNode::new_text(
                TextContent::new(text_str),
                class,
                id,
                on_click,
                on_hover,
                tooltip,
            )))
        })?,
    )?;

    vdom.set(
        "progress",
        lua.create_function(|lua, table: mlua::Table| {
            let value_num = table.get::<Option<f32>>("value")?.unwrap_or(0.0);
            let orientation_str = table
                .get::<Option<String>>("orientation")?
                .unwrap_or_default();
            let orientation = match orientation_str.to_lowercase().as_str() {
                "vertical" => Orientation::Vertical,
                _ => Orientation::Horizontal,
            };
            let (class, id, on_click, on_hover, tooltip) = parse_common_props(lua, &table)?;
            Ok(LuaVNode(VNode::new_progress(
                ProgressValue::new(value_num).unwrap_or_default(),
                orientation,
                class,
                id,
                on_click,
                on_hover,
                tooltip,
            )))
        })?,
    )?;

    vdom.set(
        "rect",
        lua.create_function(|lua, table: mlua::Table| {
            let (class, id, on_click, on_hover, tooltip) = parse_common_props(lua, &table)?;
            Ok(LuaVNode(VNode::new_rect(
                class, id, on_click, on_hover, tooltip,
            )))
        })?,
    )?;

    vdom.set(
        "image",
        lua.create_function(|lua, table: mlua::Table| {
            let (data, pixel_size) = parse_image_data_and_size(lua, &table)?;
            let (class, id, _, _, tooltip) = parse_common_props(lua, &table)?;
            Ok(LuaVNode(VNode::new_image(
                data, pixel_size, class, id, tooltip,
            )))
        })?,
    )?;

    vdom.set(
        "module",
        lua.create_function(|lua, table: mlua::Table| {
            let name_str = table.get::<String>("name")?;
            let instance_id_str = table.get::<Option<String>>("instance_id")?;
            let options = parse_module_options(lua, &table)?;
            let (class, id, on_click, on_hover, tooltip) = parse_common_props(lua, &table)?;
            Ok(LuaVNode(VNode::new_module(
                ModuleName::new(name_str),
                instance_id_str.map(ModuleInstanceId::new),
                options,
                class,
                id,
                on_click,
                on_hover,
                tooltip,
            )))
        })?,
    )?;

    lua.globals().set("vdom", vdom)?;
    Ok(())
}

pub struct LuaModule {
    lua: Mutex<Lua>,
    source: String,
    name: String,
    cached_subs: Vec<SignalKind>,
    cached_dbus_subs: Vec<DBusSubscription>,
    cached_styles: Vec<crate::features::styling::domain::StyleSheetName>,
}

impl LuaModule {
    #[must_use]
    pub fn new(name: String, source: String) -> Self {
        let lua = Lua::new();
        let _ = register_vdom_dsl(&lua);
        Self {
            lua: Mutex::new(lua),
            source,
            name,
            cached_subs: Vec::new(),
            cached_dbus_subs: Vec::new(),
            cached_styles: Vec::new(),
        }
    }

    #[cfg(test)]
    #[must_use]
    pub fn built_in(name: &str) -> Option<Self> {
        LuaScriptLoader::load_built_in(name).map(|source| Self::new(name.to_string(), source))
    }

    fn evaluate_metadata(
        lua: &Lua,
        module_name: &str,
    ) -> (
        Vec<SignalKind>,
        Vec<DBusSubscription>,
        Vec<crate::features::styling::domain::StyleSheetName>,
    ) {
        let globals = lua.globals();
        let mut subs = Vec::new();
        let mut dbus_subs = Vec::new();
        let mut styles = Vec::new();

        if let Ok(meta_fn) = globals.get::<Function>("metadata")
            && let Ok(result) = meta_fn.call::<mlua::Value>(())
            && let mlua::Value::Table(t) = result
        {
            if let Ok(subs_val) = t.get::<mlua::Value>("subscriptions")
                && let mlua::Value::Table(subs_table) = subs_val
            {
                Self::parse_subscriptions_table(&subs_table, &mut subs, &mut dbus_subs);
            }

            if let Ok(styles_val) = t.get::<mlua::Value>("styles")
                && let mlua::Value::Table(styles_table) = styles_val
            {
                for (_, val) in styles_table.pairs::<mlua::Value, mlua::Value>().flatten() {
                    if let mlua::Value::String(s) = val
                        && let Ok(s_str) = s.to_str()
                        && let Ok(sheet) =
                            crate::features::styling::domain::StyleSheetName::new(s_str.as_ref())
                    {
                        styles.push(sheet);
                    }
                }
            }
        } else if let Ok(subs_fn) = globals.get::<Function>("subscriptions")
            && let Ok(result) = subs_fn.call::<mlua::Value>(())
            && let mlua::Value::Table(t) = result
        {
            Self::parse_subscriptions_table(&t, &mut subs, &mut dbus_subs);
        }

        if styles.is_empty()
            && let Ok(default_sheet) =
                crate::features::styling::domain::StyleSheetName::new(module_name)
        {
            styles.push(default_sheet);
        }

        (subs, dbus_subs, styles)
    }

    fn parse_subscriptions_table(
        t: &mlua::Table,
        subs: &mut Vec<SignalKind>,
        dbus_subs: &mut Vec<DBusSubscription>,
    ) {
        for (_, val) in t.pairs::<mlua::Value, mlua::Value>().flatten() {
            if let mlua::Value::String(s) = &val {
                if let Ok(s_str) = s.to_str() {
                    match s_str.as_ref() {
                        "time" => subs.push(SignalKind::Time),
                        "hyprland" => subs.push(SignalKind::Hyprland),
                        "systray" => subs.push(SignalKind::Systray),
                        "metrics" => subs.push(SignalKind::Metrics),
                        "mpris" => subs.push(SignalKind::Mpris),
                        _ => {}
                    }
                }
            } else if let mlua::Value::Table(dbus_sub) = &val
                && let Ok(typ) = dbus_sub.get::<String>("type")
                && typ == "dbus"
            {
                let bus_str = dbus_sub
                    .get::<String>("bus")
                    .unwrap_or_else(|_| "session".to_string());
                let bus = if bus_str == "system" {
                    BusType::System
                } else {
                    BusType::Session
                };
                subs.push(SignalKind::DBus);
                dbus_subs.push(DBusSubscription::new(
                    bus,
                    dbus_sub
                        .get::<String>("destination")
                        .ok()
                        .map(crate::shared::dbus::domain::Destination::new),
                    dbus_sub
                        .get::<String>("path")
                        .ok()
                        .map(crate::shared::dbus::domain::Path::new),
                    dbus_sub
                        .get::<String>("interface")
                        .ok()
                        .map(crate::shared::dbus::domain::Interface::new),
                    dbus_sub
                        .get::<String>("member")
                        .ok()
                        .map(crate::shared::dbus::domain::Member::new),
                ));
            }
        }
    }
}

impl AnyModulePort for LuaModule {
    #[allow(clippy::significant_drop_tightening)]
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

            let root_config = full_config.root();

            // Expose root config
            let root_config_table = lua
                .create_table()
                .map_err(|e| ModuleInitError::ScriptError(e.to_string()))?;
            root_config_table
                .set("name", root_config.name().as_str())
                .map_err(|e| ModuleInitError::ScriptError(e.to_string()))?;
            root_config_table
                .set("height", root_config.height().value())
                .map_err(|e| ModuleInitError::ScriptError(e.to_string()))?;
            globals
                .set("root_config", root_config_table)
                .map_err(|e| ModuleInitError::ScriptError(e.to_string()))?;

            // Expose module config options using mlua's serde support
            let options_lua = lua.to_value(config.options()).map_err(|e| {
                ModuleInitError::ConfigError(format!("Failed to convert config to Lua: {e}"))
            })?;
            globals
                .set("config", options_lua)
                .map_err(|e| ModuleInitError::ScriptError(e.to_string()))?;

            // Load the script
            lua.load(&self.source)
                .set_name(&self.name)
                .exec()
                .map_err(|e| {
                    ModuleInitError::ScriptError(format!("Lua load error in {}: {e}", self.name))
                })?;

            // Call init if it exists
            if let Ok(init_fn) = globals.get::<mlua::Function>("init") {
                init_fn.call::<()>(()).map_err(|e| {
                    ModuleInitError::ScriptError(format!("Lua init error in {}: {e}", self.name))
                })?;
            }

            Self::evaluate_metadata(&lua, &self.name)
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

    #[allow(clippy::significant_drop_tightening)]
    fn render(&self, monitor: &MonitorId) -> crate::features::vdom::domain::VNode {
        let t0 = std::time::Instant::now();
        let lua = self
            .lua
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let globals = lua.globals();
        let lua_monitor = LuaMonitor(monitor.clone());

        if let Ok(render_fn) = globals.get::<mlua::Function>("render") {
            match render_fn.call::<mlua::Value>(lua_monitor) {
                Ok(val) => {
                    let vnode = value_to_vnode(&lua, val);
                    match vnode {
                        Ok(node) => {
                            tracing::debug!(
                                module = %self.name,
                                monitor = %monitor,
                                duration_ms = t0.elapsed().as_millis(),
                                duration_micros = t0.elapsed().as_micros(),
                                "Lua render completed successfully"
                            );
                            return node;
                        }
                        Err(e) => {
                            tracing::error!(
                                module = %self.name,
                                monitor = %monitor,
                                err = ?e,
                                "Failed to convert Lua return value to VNode"
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        module = %self.name,
                        monitor = %monitor,
                        err = ?e,
                        "Lua render_fn execution failed"
                    );
                }
            }
        }

        crate::features::vdom::domain::VNode::new_flex(vec![], None, None, None, None, None)
    }

    #[allow(clippy::significant_drop_tightening)]
    fn call_function(
        &mut self,
        name: &crate::shared::primitives::FunctionName,
    ) -> Result<(), ModuleInitError> {
        let lua = self
            .lua
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let globals = lua.globals();

        globals
            .get::<mlua::Function>(name.as_str())
            .map_or(Ok(()), |func| {
                func.call::<()>(()).map_err(|e| {
                    ModuleInitError::ScriptError(format!("Failed to call function '{name}': {e}"))
                })
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::systray::domain::{
        Destination, ObjectPath, SystrayId, SystrayItem, SystrayState, SystrayStatus, Title,
    };
    use crate::shared::config::domain::ModuleConfig;
    use crate::shared::primitives::MonitorId;

    #[test]
    fn test_systray_missing_icon_regression() {
        let mut module = LuaModule::built_in("systray").expect("Failed to load systray module");
        let module_config = ModuleConfig::new(
            "systray".into(),
            true,
            crate::shared::config::domain::EngineSelection::Auto,
            crate::shared::primitives::ModuleOptions::default(),
        );
        let config = crate::shared::config::domain::Config::default();

        module.init(&module_config, &config).expect("Init failed");

        let hub = SignalHub::new(crate::shared::config::domain::Config::default());
        let item = SystrayItem::new(
            crate::features::systray::domain::CreateSystrayItemCommand::new(
                SystrayId::new("test_systray"),
                Destination::new("dest"),
                ObjectPath::new("/path"),
                Title::new("Test Systray"),
                SystrayStatus::Active,
                None,
                None,
                crate::features::systray::domain::SystrayCategory::ApplicationStatus,
                crate::features::systray::domain::ItemIsMenu::new(false),
            ),
        );

        let mut map = std::collections::BTreeMap::new();
        map.insert(item.id().clone(), item);
        hub.systray_tx().send(SystrayState::new(map)).unwrap();

        let subs = module.subscriptions().to_vec();
        module.refresh(&hub, &subs);

        let layout = module.render(&MonitorId::new("DP-1"));

        // Assert it returns a flex with a single child (the systray item)
        assert_eq!(layout.tag(), crate::features::vdom::domain::NodeTag::Flex);
        assert_eq!(layout.children().len(), 1);
        let item_node = &layout.children()[0];

        // The item node itself should be a flex containing a rect (icon) and text (title)
        assert_eq!(
            item_node.tag(),
            crate::features::vdom::domain::NodeTag::Flex
        );
        assert_eq!(item_node.children().len(), 2);
        assert_eq!(
            item_node.children()[0].tag(),
            crate::features::vdom::domain::NodeTag::Rect
        );
        assert_eq!(
            item_node.children()[1].tag(),
            crate::features::vdom::domain::NodeTag::Text
        );
    }

    #[test]
    fn test_systray_with_icon_renders_image() {
        let mut module = LuaModule::built_in("systray").expect("Failed to load systray module");
        let module_config = ModuleConfig::new(
            "systray".into(),
            true,
            crate::shared::config::domain::EngineSelection::Auto,
            crate::shared::primitives::ModuleOptions::default(),
        );
        let config = crate::shared::config::domain::Config::default();
        module.init(&module_config, &config).expect("Init failed");

        let hub = SignalHub::new(crate::shared::config::domain::Config::default());
        let icon_img = crate::features::systray::domain::IconImage::new(
            vec![255; 16 * 16 * 4],
            crate::shared::primitives::geometry::Size::new(16, 16),
        );
        let icon = crate::features::systray::domain::SystrayIcon::new(
            Some(crate::features::systray::domain::IconName::new("test-icon")),
            Some(icon_img),
        );

        let item = SystrayItem::new(
            crate::features::systray::domain::CreateSystrayItemCommand::new(
                SystrayId::new("test_systray"),
                Destination::new("dest"),
                ObjectPath::new("/path"),
                Title::new("Test Systray"),
                SystrayStatus::Active,
                icon,
                None,
                crate::features::systray::domain::SystrayCategory::ApplicationStatus,
                crate::features::systray::domain::ItemIsMenu::new(false),
            ),
        );

        let mut map = std::collections::BTreeMap::new();
        map.insert(item.id().clone(), item);
        hub.systray_tx().send(SystrayState::new(map)).unwrap();

        let subs = module.subscriptions().to_vec();
        module.refresh(&hub, &subs);

        let layout = module.render(&MonitorId::new("DP-1"));
        assert_eq!(layout.tag(), crate::features::vdom::domain::NodeTag::Flex);
        assert_eq!(layout.children().len(), 1);
        let item_node = &layout.children()[0];
        assert_eq!(
            item_node.tag(),
            crate::features::vdom::domain::NodeTag::Flex
        );
        assert_eq!(item_node.children().len(), 2);
        assert_eq!(
            item_node.children()[0].tag(),
            crate::features::vdom::domain::NodeTag::Image
        );
        assert_eq!(
            item_node.children()[1].tag(),
            crate::features::vdom::domain::NodeTag::Text
        );
    }

    #[test]
    fn test_vdom_dsl_constructors() {
        let lua = Lua::new();
        register_vdom_dsl(&lua).expect("DSL registration failed");

        let script = r#"
            local txt = vdom.text({ text = "hello", class = "greeting" })
            local rect = vdom.rect({ class = "box" })
            local prog = vdom.progress({ value = 0.75, orientation = "vertical" })
            local flex = vdom.flex({
                class = "root",
                children = { txt, rect, prog }
            })
            return flex
        "#;
        let val = lua.load(script).eval::<mlua::Value>().expect("Eval failed");
        let vnode = value_to_vnode(&lua, val).expect("Conversion failed");
        assert_eq!(vnode.tag(), crate::features::vdom::domain::NodeTag::Flex);
        assert_eq!(vnode.children().len(), 3);
        assert_eq!(
            vnode.children()[0].tag(),
            crate::features::vdom::domain::NodeTag::Text
        );
        assert_eq!(
            vnode.children()[1].tag(),
            crate::features::vdom::domain::NodeTag::Rect
        );
        assert_eq!(
            vnode.children()[2].tag(),
            crate::features::vdom::domain::NodeTag::Progress
        );
    }
}
