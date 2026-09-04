use crate::features::module_runtime::ports::{AnyModulePort, ModuleInitError};
use crate::features::styling::domain::{ClassNameList, ElementId, Orientation, ProgressValue};
use crate::features::vdom::domain::{ClickHandlers, TextContent, UiAction, VNode};
use crate::shared::config::domain::ModuleConfig;
use crate::shared::dbus::domain::{BusType, DBusSubscription};
use crate::shared::events::core::PointerButton;
use crate::shared::events::signals::{SignalHub, SignalKind};
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::{
    BinaryData, ModuleInstanceId, ModuleName, ModuleOptions, MonitorId, ScriptMonitorInfo,
};
use mlua::{Function, Lua, LuaSerdeExt, UserData, UserDataFields, UserDataMethods};
use std::sync::Mutex;

#[derive(Clone)]
pub struct LuaVNode(pub VNode);

impl UserData for LuaVNode {}

#[derive(Clone)]
pub struct LuaMonitor(pub ScriptMonitorInfo);

impl UserData for LuaMonitor {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("id", |_, this| Ok(this.0.id().as_str().to_string()));
        fields.add_field_method_get("name", |_, this| Ok(this.0.name().to_string()));
        fields.add_field_method_get("width", |_, this| Ok(this.0.size().width()));
        fields.add_field_method_get("height", |_, this| Ok(this.0.size().height()));
        fields.add_field_method_get("scale", |_, this| Ok(this.0.scale().value()));
        fields.add_field_method_get("is_focused", |_, this| Ok(this.0.is_focused()));
        fields.add_field_method_get("active_workspace_id", |_, this| {
            Ok(this.0.active_workspace_id())
        });
        fields.add_field_method_get("special_workspace_id", |_, this| {
            Ok(this.0.special_workspace_id())
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::ToString, |_, this, ()| {
            Ok(this.0.id().as_str().to_string())
        });
    }
}

#[cfg(test)]
pub struct LuaScriptLoader;

#[cfg(test)]
impl LuaScriptLoader {
    #[must_use]
    pub fn load_built_in(name: &str) -> Option<String> {
        match name {
            "clock" => Some(include_str!("../../../../assets/widgets/clock.lua").to_string()),
            "calendar" => Some(include_str!("../../../../assets/widgets/calendar.lua").to_string()),
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
        let signals_table = if let Some(t) = globals.get::<Option<mlua::Table>>("signals")? {
            t
        } else {
            let t = lua.create_table()?;
            globals.set("signals", t.clone())?;
            t
        };
        let mut dbus_handled = false;

        for signal in changed {
            match signal {
                SignalKind::Time => {
                    let time = *hub.time_rx().borrow();
                    signals_table.set("time", time.to_rfc3339())?;
                }
                SignalKind::Hyprland => {
                    let hypr = hub.hyprland_rx().borrow().clone();
                    if let Ok(val) = lua.to_value(&hypr) {
                        signals_table.set("hyprland", val)?;
                    }
                }
                SignalKind::DBus if !dbus_handled => {
                    let dbus_state = hub.dbus_rx().borrow().clone();
                    if let Ok(val) = lua.to_value(&dbus_state.properties()) {
                        signals_table.set("dbus", val)?;
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
                            signals_table.set("systray", val)?;
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
                        signals_table.set("metrics", val)?;
                    }
                }
                SignalKind::Mpris => {
                    let mpris = hub.mpris_rx().borrow().clone();
                    if let Ok(val) = lua.to_value(&mpris) {
                        let _ = signals_table.set("mpris", val);
                    }
                }
                SignalKind::DBus => {}
            }
        }

        // Update raw monitors
        let monitor_infos = hub.get_monitor_infos();
        let mon_table = lua.create_table()?;
        let mut focused_mon = None;
        for (i, info) in monitor_infos.into_iter().enumerate() {
            if info.is_focused() {
                focused_mon = Some(LuaMonitor(info.clone()));
            }
            mon_table.set(i.saturating_add(1), LuaMonitor(info))?;
        }
        globals.set("_raw_monitors", mon_table)?;
        globals.set("_raw_focused_monitor", focused_mon)?;

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

fn parse_ui_action(lua: &Lua, val: Option<mlua::Value>) -> Option<UiAction> {
    match val {
        Some(mlua::Value::Table(t)) => {
            let cmd: Result<UiAction, _> = lua.from_value(mlua::Value::Table(t));
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

fn parse_click_handlers(lua: &Lua, val: Option<mlua::Value>) -> Option<ClickHandlers> {
    let Some(mlua::Value::Table(t)) = val else {
        return None;
    };

    let mut handlers = ClickHandlers::new();

    for (k, v) in t.pairs::<mlua::Value, mlua::Value>().flatten() {
        let button_opt = match k {
            mlua::Value::String(s) => s
                .to_str()
                .ok()
                .as_deref()
                .and_then(PointerButton::from_name),
            mlua::Value::Integer(i) => u32::try_from(i).ok().map(PointerButton::from_raw),
            mlua::Value::Number(n) => {
                if n >= 0.0 && n.fract() == 0.0 && n <= f64::from(u32::MAX) {
                    #[allow(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        clippy::as_conversions
                    )]
                    Some(PointerButton::from_raw(n as u32))
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(btn) = button_opt
            && let Some(cmd) = parse_ui_action(lua, Some(v))
        {
            handlers.insert(btn, cmd);
        }
    }

    if !handlers.is_empty() {
        return Some(handlers);
    }

    parse_ui_action(lua, Some(mlua::Value::Table(t))).map(ClickHandlers::from_single)
}

type CommonProps = (
    Option<ClassNameList>,
    Option<ElementId>,
    Option<ClickHandlers>,
    Option<UiAction>,
    Option<Box<VNode>>,
    Option<Box<VNode>>,
);

fn parse_common_props(lua: &Lua, table: &mlua::Table) -> mlua::Result<CommonProps> {
    let class = table
        .get::<Option<String>>("class")?
        .and_then(|s| ClassNameList::parse(&s).ok());
    let id = table
        .get::<Option<String>>("id")?
        .and_then(|s| ElementId::new(s).ok());
    let on_click = parse_click_handlers(lua, table.get::<Option<mlua::Value>>("on_click")?);
    let on_hover = parse_ui_action(lua, table.get::<Option<mlua::Value>>("on_hover")?);
    let tooltip = match table.get::<Option<mlua::Value>>("tooltip")? {
        Some(mlua::Value::Nil) | None => None,
        Some(val) => Some(Box::new(value_to_vnode(lua, val)?)),
    };
    let popup = match table.get::<Option<mlua::Value>>("popup")? {
        Some(mlua::Value::Nil) | None => None,
        Some(val) => Some(Box::new(value_to_vnode(lua, val)?)),
    };
    Ok((class, id, on_click, on_hover, tooltip, popup))
}

fn parse_table_children(lua: &Lua, table: &mlua::Table) -> mlua::Result<Vec<VNode>> {
    let mut children_vec = Vec::new();
    if let Ok(children) = table.get::<mlua::Table>("children") {
        for pair in children.sequence_values::<mlua::Value>() {
            let child_val = pair?;
            children_vec.push(value_to_vnode(lua, child_val)?);
        }
    }
    Ok(children_vec)
}

fn table_to_vnode(lua: &Lua, table: mlua::Table) -> mlua::Result<VNode> {
    let typ: String = table.get::<Option<String>>("type")?.unwrap_or_default();
    let (class, id, on_click, on_hover, tooltip, popup) = parse_common_props(lua, &table)?;

    let mut node = match typ.as_str() {
        "flex" | "" => VNode::new_flex(
            parse_table_children(lua, &table)?,
            class,
            id,
            on_click,
            on_hover,
            tooltip,
        ),
        "grid" => VNode::new_grid(
            parse_table_children(lua, &table)?,
            class,
            id,
            on_click,
            on_hover,
            tooltip,
        ),
        "text" => {
            let text_str = parse_text_content(&table);
            VNode::new_text(
                TextContent::new(text_str),
                class,
                id,
                on_click,
                on_hover,
                tooltip,
            )
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
            VNode::new_progress(
                ProgressValue::new(value_num).unwrap_or_default(),
                orientation,
                class,
                id,
                on_click,
                on_hover,
                tooltip,
            )
        }
        "rect" => VNode::new_rect(class, id, on_click, on_hover, tooltip),
        "image" => {
            let (data, pixel_size) = parse_image_data_and_size(lua, &table)?;
            VNode::new_image(data, pixel_size, class, id, tooltip)
        }
        "module" => {
            let name_str = table.get::<String>("name")?;
            let instance_id_str = table.get::<Option<String>>("instance_id")?;
            let options = parse_module_options(lua, &table)?;
            VNode::new_module(
                ModuleName::new(name_str),
                instance_id_str.map(ModuleInstanceId::new),
                options,
                class,
                id,
                on_click,
                on_hover,
                tooltip,
            )
        }
        _ => return lua.from_value::<VNode>(mlua::Value::Table(table)),
    };

    if let Some(p) = popup {
        node = node.with_popup(p);
    }
    Ok(node)
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
        mlua::Value::Table(table) => table_to_vnode(lua, table),
        other => lua.from_value::<VNode>(other),
    }
}

fn parse_flex_node(lua: &Lua, val: mlua::Value) -> mlua::Result<VNode> {
    let mlua::Value::Table(table) = val else {
        return Ok(VNode::new_flex(Vec::new(), None, None, None, None, None));
    };

    let is_full_spec = table.contains_key("type")?
        || table.contains_key("class")?
        || table.contains_key("children")?
        || table.contains_key("id")?
        || table.contains_key("on_click")?
        || table.contains_key("on_hover")?
        || table.contains_key("tooltip")?
        || table.contains_key("popup")?;

    if is_full_spec {
        let children_vec = parse_table_children(lua, &table)?;
        let (class, id, on_click, on_hover, tooltip, popup) = parse_common_props(lua, &table)?;
        let mut node = VNode::new_flex(children_vec, class, id, on_click, on_hover, tooltip);
        if let Some(p) = popup {
            node = node.with_popup(p);
        }
        return Ok(node);
    }

    let mut children_vec = Vec::new();
    for pair in table.sequence_values::<mlua::Value>() {
        children_vec.push(value_to_vnode(lua, pair?)?);
    }
    Ok(VNode::new_flex(children_vec, None, None, None, None, None))
}

fn parse_grid_node(lua: &Lua, val: mlua::Value) -> mlua::Result<VNode> {
    let mlua::Value::Table(table) = val else {
        return Ok(VNode::new_grid(Vec::new(), None, None, None, None, None));
    };

    let is_full_spec = table.contains_key("type")?
        || table.contains_key("class")?
        || table.contains_key("children")?
        || table.contains_key("id")?
        || table.contains_key("on_click")?
        || table.contains_key("on_hover")?
        || table.contains_key("tooltip")?
        || table.contains_key("popup")?;

    if is_full_spec {
        let children_vec = parse_table_children(lua, &table)?;
        let (class, id, on_click, on_hover, tooltip, popup) = parse_common_props(lua, &table)?;
        let mut node = VNode::new_grid(children_vec, class, id, on_click, on_hover, tooltip);
        if let Some(p) = popup {
            node = node.with_popup(p);
        }
        return Ok(node);
    }

    let mut children_vec = Vec::new();
    for pair in table.sequence_values::<mlua::Value>() {
        children_vec.push(value_to_vnode(lua, pair?)?);
    }
    Ok(VNode::new_grid(children_vec, None, None, None, None, None))
}

fn parse_text_node(
    lua: &Lua,
    val: mlua::Value,
    class_opt: Option<String>,
) -> mlua::Result<VNode> {
    match val {
        mlua::Value::Table(table) => {
            let text_str = parse_text_content(&table);
            let (class, id, on_click, on_hover, tooltip, popup) = parse_common_props(lua, &table)?;
            let mut node = VNode::new_text(
                TextContent::new(text_str),
                class,
                id,
                on_click,
                on_hover,
                tooltip,
            );
            if let Some(p) = popup {
                node = node.with_popup(p);
            }
            Ok(node)
        }
        mlua::Value::String(s) => {
            let text_str = s.to_str().map_or_else(|_| String::new(), |b| b.to_string());
            let class = class_opt.and_then(|c| ClassNameList::parse(&c).ok());
            Ok(VNode::new_text(
                TextContent::new(text_str),
                class,
                None,
                None,
                None,
                None,
            ))
        }
        mlua::Value::Integer(i) => {
            let class = class_opt.and_then(|c| ClassNameList::parse(&c).ok());
            Ok(VNode::new_text(
                TextContent::new(i.to_string()),
                class,
                None,
                None,
                None,
                None,
            ))
        }
        mlua::Value::Number(n) => {
            let class = class_opt.and_then(|c| ClassNameList::parse(&c).ok());
            Ok(VNode::new_text(
                TextContent::new(n.to_string()),
                class,
                None,
                None,
                None,
                None,
            ))
        }
        _ => Ok(VNode::new_text(
            TextContent::new(String::new()),
            None,
            None,
            None,
            None,
            None,
        )),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn parse_progress_node(
    lua: &Lua,
    val: mlua::Value,
    orientation_opt: Option<String>,
    class_opt: Option<String>,
) -> mlua::Result<VNode> {
    match val {
        mlua::Value::Table(table) => {
            let value_num = table.get::<Option<f32>>("value")?.unwrap_or(0.0);
            let orientation_str = table
                .get::<Option<String>>("orientation")?
                .unwrap_or_default();
            let orientation = match orientation_str.to_lowercase().as_str() {
                "vertical" => Orientation::Vertical,
                _ => Orientation::Horizontal,
            };
            let (class, id, on_click, on_hover, tooltip, popup) = parse_common_props(lua, &table)?;
            let mut node = VNode::new_progress(
                ProgressValue::new(value_num).unwrap_or_default(),
                orientation,
                class,
                id,
                on_click,
                on_hover,
                tooltip,
            );
            if let Some(p) = popup {
                node = node.with_popup(p);
            }
            Ok(node)
        }
        mlua::Value::Number(n) => {
            #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
            let val_f32 = n as f32;
            let orientation = match orientation_opt
                .as_deref()
                .unwrap_or("horizontal")
                .to_lowercase()
                .as_str()
            {
                "vertical" => Orientation::Vertical,
                _ => Orientation::Horizontal,
            };
            let class = class_opt.and_then(|c| ClassNameList::parse(&c).ok());
            Ok(VNode::new_progress(
                ProgressValue::new(val_f32).unwrap_or_default(),
                orientation,
                class,
                None,
                None,
                None,
                None,
            ))
        }
        mlua::Value::Integer(i) => {
            #[allow(
                clippy::as_conversions,
                clippy::cast_possible_truncation,
                clippy::cast_precision_loss
            )]
            let val_f32 = i as f32;
            let orientation = match orientation_opt
                .as_deref()
                .unwrap_or("horizontal")
                .to_lowercase()
                .as_str()
            {
                "vertical" => Orientation::Vertical,
                _ => Orientation::Horizontal,
            };
            let class = class_opt.and_then(|c| ClassNameList::parse(&c).ok());
            Ok(VNode::new_progress(
                ProgressValue::new(val_f32).unwrap_or_default(),
                orientation,
                class,
                None,
                None,
                None,
                None,
            ))
        }
        _ => Ok(VNode::new_progress(
            ProgressValue::default(),
            Orientation::Horizontal,
            None,
            None,
            None,
            None,
            None,
        )),
    }
}

fn parse_rect_node(lua: &Lua, val: Option<mlua::Value>) -> mlua::Result<VNode> {
    match val {
        Some(mlua::Value::Table(table)) => {
            let (class, id, on_click, on_hover, tooltip, popup) = parse_common_props(lua, &table)?;
            let mut node = VNode::new_rect(class, id, on_click, on_hover, tooltip);
            if let Some(p) = popup {
                node = node.with_popup(p);
            }
            Ok(node)
        }
        Some(mlua::Value::String(s)) => {
            let class = s.to_str().ok().and_then(|c| ClassNameList::parse(c.as_ref()).ok());
            Ok(VNode::new_rect(class, None, None, None, None))
        }
        _ => Ok(VNode::new_rect(None, None, None, None, None)),
    }
}

fn parse_module_node(lua: &Lua, val: mlua::Value) -> mlua::Result<VNode> {
    match val {
        mlua::Value::Table(table) => {
            let name_str = table.get::<String>("name")?;
            let instance_id_str = table.get::<Option<String>>("instance_id")?;
            let options = parse_module_options(lua, &table)?;
            let (class, id, on_click, on_hover, tooltip, popup) = parse_common_props(lua, &table)?;
            let mut node = VNode::new_module(
                ModuleName::new(name_str),
                instance_id_str.map(ModuleInstanceId::new),
                options,
                class,
                id,
                on_click,
                on_hover,
                tooltip,
            );
            if let Some(p) = popup {
                node = node.with_popup(p);
            }
            Ok(node)
        }
        mlua::Value::String(s) => {
            let name_str = s.to_str().map_or_else(|_| String::new(), |b| b.to_string());
            Ok(VNode::new_module(
                ModuleName::new(name_str),
                None,
                ModuleOptions::default(),
                None,
                None,
                None,
                None,
                None,
            ))
        }
        _ => Ok(VNode::new_module(
            ModuleName::new(String::new()),
            None,
            ModuleOptions::default(),
            None,
            None,
            None,
            None,
            None,
        )),
    }
}

macro_rules! set_table_aliases {
    ($table:expr, $value:expr, $($key:literal),*) => {
        $(
            $table.set($key, $value.clone())?;
        )*
    };
}

macro_rules! register_lua_log_methods {
    ($lua:expr, $table:expr, $(($name:literal, $level:ident)),*) => {
        $(
            $table.set(
                $name,
                $lua.create_function(|_, msg: String| {
                    tracing::$level!("{msg}");
                    Ok(())
                })?,
            )?;
        )*
    };
}

fn create_ui_action_table(lua: &Lua) -> mlua::Result<mlua::Table> {
    let action_table = lua.create_table()?;

    action_table.set(
        "exec",
        lua.create_function(|lua, cmd: String| {
            let t = lua.create_table()?;
            t.set("Exec", cmd)?;
            Ok(t)
        })?,
    )?;

    let call_fn = lua.create_function(|lua, name: String| {
        let t = lua.create_table()?;
        t.set("ScriptCall", name)?;
        Ok(t)
    })?;

    set_table_aliases!(
        action_table,
        call_fn,
        "call",
        "script_call",
        "call_script",
        "func"
    );

    action_table.set(
        "systray",
        lua.create_function(|lua, (id, action): (mlua::Value, Option<String>)| {
            let id_str = match id {
                mlua::Value::String(s) => {
                    s.to_str().map_or_else(|_| String::new(), |b| b.to_string())
                }
                mlua::Value::Integer(i) => i.to_string(),
                mlua::Value::Number(n) => n.to_string(),
                _ => String::new(),
            };
            let action_str = action.unwrap_or_else(|| "Primary".to_string());
            let t = lua.create_table()?;
            let sub = lua.create_table()?;
            sub.set("id", id_str)?;
            sub.set("action", action_str)?;
            t.set("SystrayAction", sub)?;
            Ok(t)
        })?,
    )?;

    Ok(action_table)
}

fn create_ui_element_table(lua: &Lua) -> mlua::Result<mlua::Table> {
    let element_table = lua.create_table()?;

    let flex_fn = lua.create_function(|lua, val: mlua::Value| {
        let node = parse_flex_node(lua, val)?;
        Ok(LuaVNode(node))
    })?;

    let grid_fn = lua.create_function(|lua, val: mlua::Value| {
        let node = parse_grid_node(lua, val)?;
        Ok(LuaVNode(node))
    })?;

    let text_fn = lua.create_function(
        |lua, (val, class_opt): (mlua::Value, Option<String>)| {
            let node = parse_text_node(lua, val, class_opt)?;
            Ok(LuaVNode(node))
        },
    )?;

    let progress_fn = lua.create_function(
        |lua, (val, orientation_opt, class_opt): (mlua::Value, Option<String>, Option<String>)| {
            let node = parse_progress_node(lua, val, orientation_opt, class_opt)?;
            Ok(LuaVNode(node))
        },
    )?;

    let rect_fn = lua.create_function(|lua, val: Option<mlua::Value>| {
        let node = parse_rect_node(lua, val)?;
        Ok(LuaVNode(node))
    })?;

    let image_fn = lua.create_function(|lua, table: mlua::Table| {
        let (data, pixel_size) = parse_image_data_and_size(lua, &table)?;
        let (class, id, _, _, tooltip, popup) = parse_common_props(lua, &table)?;
        let mut node = VNode::new_image(data, pixel_size, class, id, tooltip);
        if let Some(p) = popup {
            node = node.with_popup(p);
        }
        Ok(LuaVNode(node))
    })?;

    let module_fn = lua.create_function(|lua, val: mlua::Value| {
        let node = parse_module_node(lua, val)?;
        Ok(LuaVNode(node))
    })?;

    element_table.set("flex", flex_fn)?;
    element_table.set("grid", grid_fn)?;
    element_table.set("text", text_fn)?;
    element_table.set("progress", progress_fn)?;
    element_table.set("rect", rect_fn)?;
    element_table.set("image", image_fn)?;
    set_table_aliases!(
        element_table,
        module_fn,
        "module",
        "widget",
        "load_module"
    );

    Ok(element_table)
}

fn create_sys_log_table(lua: &Lua) -> mlua::Result<mlua::Table> {
    let log_table = lua.create_table()?;
    register_lua_log_methods!(
        lua,
        log_table,
        ("debug", debug),
        ("info", info),
        ("warn", warn),
        ("error", error)
    );
    Ok(log_table)
}

fn create_sys_monitors_table(lua: &Lua) -> mlua::Result<mlua::Table> {
    let monitors_table = lua.create_table()?;
    monitors_table.set(
        "all",
        lua.create_function(|lua, ()| {
            let globals = lua.globals();
            let raw_monitors = globals.get::<Option<mlua::Table>>("_raw_monitors")?;
            if let Some(t) = raw_monitors {
                return Ok(t);
            }
            lua.create_table()
        })?,
    )?;
    monitors_table.set(
        "focused",
        lua.create_function(|lua, ()| {
            let globals = lua.globals();
            let raw_focused = globals.get::<Option<mlua::Value>>("_raw_focused_monitor")?;
            Ok(raw_focused)
        })?,
    )?;
    monitors_table.set(
        "current",
        lua.create_function(|lua, ()| {
            let globals = lua.globals();
            let raw_focused = globals.get::<Option<mlua::Value>>("_raw_focused_monitor")?;
            Ok(raw_focused)
        })?,
    )?;
    monitors_table.set(
        "get",
        lua.create_function(|lua, id: String| {
            let globals = lua.globals();
            let raw_monitors = globals.get::<Option<mlua::Table>>("_raw_monitors")?;
            let Some(t) = raw_monitors else {
                return Ok(None);
            };
            for pair in t.sequence_values::<mlua::Value>() {
                if let Ok(mlua::Value::UserData(ud)) = pair
                    && let Ok(mon) = ud.borrow::<LuaMonitor>()
                    && mon.0.id().as_str() == id.as_str()
                {
                    return Ok(Some(mlua::Value::UserData(ud)));
                }
            }
            Ok(None)
        })?,
    )?;
    Ok(monitors_table)
}

fn create_sys_table(lua: &Lua) -> mlua::Result<mlua::Table> {
    let sys = lua.create_table()?;

    sys.set(
        "exec",
        lua.create_function(|_, cmd: String| {
            let _ = std::process::Command::new("sh").arg("-c").arg(&cmd).spawn();
            Ok(())
        })?,
    )?;

    sys.set(
        "env",
        lua.create_function(|_, name: String| Ok(std::env::var(&name).ok()))?,
    )?;

    sys.set(
        "hostname",
        lua.create_function(|_, ()| {
            let name = std::fs::read_to_string("/etc/hostname").map_or_else(
                |_| std::env::var("HOSTNAME").unwrap_or_else(|_| "localhost".to_string()),
                |s| s.trim().to_string(),
            );
            Ok(name)
        })?,
    )?;

    let log_table = create_sys_log_table(lua)?;
    sys.set("log", log_table)?;

    let monitors_table = create_sys_monitors_table(lua)?;
    sys.set("monitors", monitors_table)?;

    Ok(sys)
}

/// Registers the `cranky`, `ui`, `signals`, `config`, and `sys` APIs in Lua globals.
///
/// # Errors
///
/// Returns `mlua::Error` if table creation or function registration fails.
pub fn register_cranky_api(lua: &Lua) -> mlua::Result<()> {
    let ui_table = lua.create_table()?;
    let element_table = create_ui_element_table(lua)?;
    let action_table = create_ui_action_table(lua)?;

    ui_table.set("element", element_table.clone())?;
    ui_table.set("action", action_table)?;
    for (k, v) in element_table.pairs::<String, mlua::Function>().flatten() {
        ui_table.set(k, v)?;
    }

    let signals_table = lua.create_table()?;
    let config_table = lua.create_table()?;
    let sys_table = create_sys_table(lua)?;

    let cranky_table = lua.create_table()?;
    cranky_table.set("ui", ui_table.clone())?;
    cranky_table.set("signals", signals_table.clone())?;
    cranky_table.set("config", config_table.clone())?;
    cranky_table.set("sys", sys_table.clone())?;

    let globals = lua.globals();
    globals.set("cranky", cranky_table)?;
    globals.set("ui", ui_table)?;
    globals.set("signals", signals_table)?;
    globals.set("config", config_table)?;
    globals.set("sys", sys_table)?;

    Ok(())
}

/// Legacy alias for `register_cranky_api`.
///
/// # Errors
///
/// Returns `mlua::Error` if table creation or function registration fails.
pub fn register_vdom_dsl(lua: &Lua) -> mlua::Result<()> {
    register_cranky_api(lua)
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

            // Set up config table
            let config_table = if let Some(t) = globals
                .get::<Option<mlua::Table>>("config")
                .map_err(|e| ModuleInitError::ScriptError(e.to_string()))?
            {
                t
            } else {
                let t = lua
                    .create_table()
                    .map_err(|e| ModuleInitError::ScriptError(e.to_string()))?;
                globals
                    .set("config", t.clone())
                    .map_err(|e| ModuleInitError::ScriptError(e.to_string()))?;
                t
            };

            let root_config_table = lua
                .create_table()
                .map_err(|e| ModuleInitError::ScriptError(e.to_string()))?;
            root_config_table
                .set("name", root_config.name().as_str())
                .map_err(|e| ModuleInitError::ScriptError(e.to_string()))?;
            root_config_table
                .set("height", root_config.height().value())
                .map_err(|e| ModuleInitError::ScriptError(e.to_string()))?;
            config_table
                .set("root", root_config_table)
                .map_err(|e| ModuleInitError::ScriptError(e.to_string()))?;

            // Expose module config options using mlua's serde support
            let options_lua = lua.to_value(config.options()).map_err(|e| {
                ModuleInitError::ConfigError(format!("Failed to convert config to Lua: {e}"))
            })?;
            config_table
                .set("module", options_lua.clone())
                .map_err(|e| ModuleInitError::ScriptError(e.to_string()))?;
            config_table
                .set("options", options_lua)
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
        register_cranky_api(&lua).expect("DSL registration failed");

        let script = r#"
            local txt = ui.text({ text = "hello", class = "greeting" })
            local rect = ui.rect({ class = "box" })
            local prog = ui.progress({ value = 0.75, orientation = "vertical" })
            local flex = ui.flex({
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

    #[test]
    fn test_lua_vnode_click_handlers() {
        let lua = Lua::new();
        register_cranky_api(&lua).expect("DSL registration failed");

        // Single shorthand
        let single_script = r#"
            return ui.text({
                text = "click me",
                on_click = { Exec = "echo single" }
            })
        "#;
        let single_val = lua.load(single_script).eval::<mlua::Value>().unwrap();
        let single_node = value_to_vnode(&lua, single_val).unwrap();
        let single_handlers = single_node.on_click().expect("on_click expected");
        assert_eq!(
            single_handlers.get(&PointerButton::Left),
            Some(&UiAction::Exec("echo single".into()))
        );
        assert_eq!(
            single_handlers.get(&PointerButton::Right),
            Some(&UiAction::Exec("echo single".into()))
        );
        assert_eq!(
            single_handlers.get(&PointerButton::Middle),
            Some(&UiAction::Exec("echo single".into()))
        );

        // Multi-button map
        let multi_script = r#"
            return ui.text({
                text = "multi click",
                on_click = {
                    left = { Exec = "echo left" },
                    right = { Exec = "echo right" },
                    side = { Exec = "echo side" },
                    [276] = { Exec = "echo extra" }
                }
            })
        "#;
        let multi_val = lua.load(multi_script).eval::<mlua::Value>().unwrap();
        let multi_node = value_to_vnode(&lua, multi_val).unwrap();
        let multi_handlers = multi_node.on_click().expect("on_click expected");
        assert_eq!(
            multi_handlers.get(&PointerButton::Left),
            Some(&UiAction::Exec("echo left".into()))
        );
        assert_eq!(
            multi_handlers.get(&PointerButton::Right),
            Some(&UiAction::Exec("echo right".into()))
        );
        assert_eq!(
            multi_handlers.get(&PointerButton::Side),
            Some(&UiAction::Exec("echo side".into()))
        );
        assert_eq!(
            multi_handlers.get(&PointerButton::Extra),
            Some(&UiAction::Exec("echo extra".into()))
        );
        assert_eq!(multi_handlers.get(&PointerButton::Middle), None);
    }

    #[test]
    fn test_lua_vnode_with_popup() {
        let lua = Lua::new();
        register_cranky_api(&lua).expect("DSL registration failed");

        let script = r#"
            return ui.flex({
                children = {
                    ui.text({
                        text = "Show Popup",
                        popup = ui.flex({
                            class = "popup-menu",
                            children = {
                                ui.text({ text = "Item 1" })
                            }
                        })
                    })
                }
            })
        "#;
        let val = lua.load(script).eval::<mlua::Value>().unwrap();
        let node = value_to_vnode(&lua, val).unwrap();
        assert_eq!(node.children().len(), 1);
        let button_node = &node.children()[0];
        assert!(button_node.popup().is_some());
        let popup = button_node.popup().unwrap();
        assert_eq!(popup.children().len(), 1);
        assert_eq!(popup.tag(), crate::features::vdom::domain::NodeTag::Flex);
    }

    #[test]
    fn test_lua_vnode_grid() {
        let lua = Lua::new();
        register_cranky_api(&lua).expect("DSL registration failed");

        let script = r#"
            return ui.grid({
                class = "my-grid",
                children = {
                    ui.text({ text = "Cell 1" }),
                    ui.text({ text = "Cell 2" })
                }
            })
        "#;
        let val = lua.load(script).eval::<mlua::Value>().unwrap();
        let node = value_to_vnode(&lua, val).unwrap();
        assert_eq!(node.tag(), crate::features::vdom::domain::NodeTag::Grid);
        assert_eq!(node.children().len(), 2);
    }

    #[test]
    fn test_calendar_lua_config_monday() {
        let mut module = LuaModule::built_in("calendar").expect("Failed to load calendar module");
        let mut opts = std::collections::HashMap::new();
        opts.insert(
            "first_day_of_week".to_string(),
            crate::shared::primitives::DynamicValue::String("monday".to_string()),
        );
        let module_config = ModuleConfig::new(
            "calendar".into(),
            true,
            crate::shared::config::domain::EngineSelection::Auto,
            crate::shared::primitives::ModuleOptions::new(opts),
        );
        let config = crate::shared::config::domain::Config::default();
        module.init(&module_config, &config).expect("Init failed");

        let hub = SignalHub::new(crate::shared::config::domain::Config::default());
        let test_time = chrono::DateTime::parse_from_rfc3339("2026-09-01T12:00:00+00:00")
            .unwrap()
            .with_timezone(&chrono::Local);
        hub.time_tx().send(test_time).unwrap();
        let subs = module.subscriptions().to_vec();
        module.refresh(&hub, &subs);

        let node = module.render(&MonitorId::new("DP-1"));
        let weekdays = &node.children()[1];
        if let crate::features::vdom::domain::VNodeKind::Text { text } =
            weekdays.children()[0].kind()
        {
            assert_eq!(
                text.as_str(),
                "Mo",
                "First weekday should be Mo for monday config"
            );
        } else {
            panic!("Expected text node for weekday");
        }
    }

    #[test]
    fn test_lua_cranky_namespaces_and_aliases() {
        let lua = Lua::new();
        register_cranky_api(&lua).expect("Registration failed");

        let script = r#"
            assert(type(cranky) == "table", "cranky must be table")
            assert(type(cranky.ui) == "table", "cranky.ui must be table")
            assert(type(cranky.ui.element) == "table", "cranky.ui.element must be table")
            assert(type(cranky.ui.action) == "table", "cranky.ui.action must be table")
            assert(type(cranky.signals) == "table", "cranky.signals must be table")
            assert(type(cranky.config) == "table", "cranky.config must be table")
            assert(type(cranky.sys) == "table", "cranky.sys must be table")

            -- Direct aliases
            assert(ui == cranky.ui, "ui must alias cranky.ui")
            assert(signals == cranky.signals, "signals must alias cranky.signals")
            assert(config == cranky.config, "config must alias cranky.config")
            assert(sys == cranky.sys, "sys must alias cranky.sys")

            -- Direct element aliases on ui
            assert(type(ui.flex) == "function", "ui.flex must be function")
            assert(type(ui.text) == "function", "ui.text must be function")
            assert(type(ui.grid) == "function", "ui.grid must be function")
            assert(type(ui.progress) == "function", "ui.progress must be function")
            assert(type(ui.rect) == "function", "ui.rect must be function")
            assert(type(ui.image) == "function", "ui.image must be function")
            assert(type(ui.module) == "function", "ui.module must be function")
            return true
        "#;
        let result = lua.load(script).eval::<bool>().expect("Script evaluation failed");
        assert!(result);
    }

    #[test]
    fn test_lua_shorthand_dsl() {
        let lua = Lua::new();
        register_cranky_api(&lua).expect("Registration failed");

        let script = r#"
            local t = ui.text("Hello World", "greeting-cls")
            local p = ui.progress(0.75, "vertical", "cpu-bar")
            local r = ui.rect("separator")
            local m = ui.module("clock")
            local root = ui.flex({ t, p, r, m })
            return root
        "#;
        let val = lua.load(script).eval::<mlua::Value>().unwrap();
        let vnode = value_to_vnode(&lua, val).unwrap();
        assert_eq!(vnode.tag(), crate::features::vdom::domain::NodeTag::Flex);
        assert_eq!(vnode.children().len(), 4);
        assert_eq!(vnode.children()[0].tag(), crate::features::vdom::domain::NodeTag::Text);
        assert_eq!(vnode.children()[1].tag(), crate::features::vdom::domain::NodeTag::Progress);
        assert_eq!(vnode.children()[2].tag(), crate::features::vdom::domain::NodeTag::Rect);
        assert_eq!(vnode.children()[3].tag(), crate::features::vdom::domain::NodeTag::Module);
    }

    #[test]
    fn test_lua_ui_actions() {
        let lua = Lua::new();
        register_cranky_api(&lua).expect("Registration failed");

        let script = r#"
            local a1 = ui.action.exec("echo 123")
            local a2 = ui.action.call("toggle_popup")
            local a3 = ui.action.systray(5, "Primary")
            return { a1 = a1, a2 = a2, a3 = a3 }
        "#;
        let table = lua.load(script).eval::<mlua::Table>().unwrap();
        let a1: mlua::Table = table.get("a1").unwrap();
        assert_eq!(a1.get::<String>("Exec").unwrap(), "echo 123");

        let a2: mlua::Table = table.get("a2").unwrap();
        assert_eq!(a2.get::<String>("ScriptCall").unwrap(), "toggle_popup");

        let a3: mlua::Table = table.get("a3").unwrap();
        let sub: mlua::Table = a3.get("SystrayAction").unwrap();
        assert_eq!(sub.get::<u32>("id").unwrap(), 5);
        assert_eq!(sub.get::<String>("action").unwrap(), "Primary");
    }

    #[test]
    fn test_lua_rich_monitor_userdata() {
        let lua = Lua::new();
        let info = crate::shared::primitives::ScriptMonitorInfo::new(
            MonitorId::new("DP-1"),
            "DP-1".to_string(),
            crate::shared::primitives::geometry::Size::new(1920, 1080),
            crate::shared::primitives::geometry::Scale::new(1.5),
            true,
            Some(1),
            Some(2),
        );
        let mon = LuaMonitor(info);
        lua.globals().set("test_mon", mon).unwrap();

        let script = r#"
            assert(test_mon.id == "DP-1", "id field")
            assert(test_mon.name == "DP-1", "name field")
            assert(test_mon.width == 1920, "width field")
            assert(test_mon.height == 1080, "height field")
            assert(test_mon.scale == 1.5, "scale field")
            assert(test_mon.is_focused == true, "is_focused field")
            assert(test_mon.active_workspace_id == 1, "active_workspace_id field")
            assert(test_mon.special_workspace_id == 2, "special_workspace_id field")
            assert(tostring(test_mon) == "DP-1", "tostring()")
            return true
        "#;
        let result = lua.load(script).eval::<bool>().unwrap();
        assert!(result);
    }
}
