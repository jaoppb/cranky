use super::dsl_container::{
    parse_flex_node, parse_grid_node, parse_module_node, parse_rect_node,
};
use super::dsl_leaf::{parse_progress_node, parse_text_node};
use super::parser_props::{
    parse_common_props, parse_image_data_and_size, parse_panel_spec, parse_popup_spec,
};
use super::userdata::{LuaPanel, LuaPopup, LuaVNode};
use crate::features::vdom::domain::VNode;
use mlua::Lua;

macro_rules! set_table_aliases {
    ($table:expr, $value:expr, $($key:literal),*) => {
        $(
            $table.set($key, $value.clone())?;
        )*
    };
}

pub(crate) fn create_ui_action_table(lua: &Lua) -> mlua::Result<mlua::Table> {
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

pub(crate) fn create_ui_element_table(lua: &Lua) -> mlua::Result<mlua::Table> {
    let element_table = lua.create_table()?;

    let flex_fn = lua.create_function(|lua, val: mlua::Value| {
        let node = parse_flex_node(lua, val)?;
        Ok(LuaVNode(node))
    })?;

    let grid_fn = lua.create_function(|lua, val: mlua::Value| {
        let node = parse_grid_node(lua, val)?;
        Ok(LuaVNode(node))
    })?;

    let text_fn = lua.create_function(|lua, (val, class_opt): (mlua::Value, Option<String>)| {
        let node = parse_text_node(lua, val, class_opt.as_deref())?;
        Ok(LuaVNode(node))
    })?;

    let progress_fn = lua.create_function(
        |lua, (val, orientation_opt, class_opt): (mlua::Value, Option<String>, Option<String>)| {
            let node = parse_progress_node(
                lua,
                val,
                orientation_opt.as_deref(),
                class_opt.as_deref(),
            )?;
            Ok(LuaVNode(node))
        },
    )?;

    let rect_fn = lua.create_function(|lua, val: Option<mlua::Value>| {
        let node = parse_rect_node(lua, val)?;
        Ok(LuaVNode(node))
    })?;

    let image_fn = lua.create_function(|lua, table: mlua::Table| {
        let (data, pixel_size) = parse_image_data_and_size(lua, &table)?;
        let (class, id, _, _, tooltip, popup, panel) = parse_common_props(lua, &table)?;
        let mut node = VNode::new_image(data, pixel_size, class, id, tooltip);
        if let Some(p) = popup {
            node = node.with_popup(p);
        }
        if let Some(p) = panel {
            node = node.with_panel(p);
        }
        Ok(LuaVNode(node))
    })?;

    let module_fn = lua.create_function(|lua, val: mlua::Value| {
        let node = parse_module_node(lua, val)?;
        Ok(LuaVNode(node))
    })?;

    let popup_fn = lua.create_function(|lua, val: mlua::Value| {
        let spec_opt = parse_popup_spec(lua, Some(val))?;
        spec_opt
            .map(LuaPopup)
            .ok_or_else(|| mlua::Error::RuntimeError("Invalid popup specification".to_string()))
    })?;

    let panel_fn = lua.create_function(|lua, val: mlua::Value| {
        let spec_opt = parse_panel_spec(lua, Some(val))?;
        spec_opt
            .map(LuaPanel)
            .ok_or_else(|| mlua::Error::RuntimeError("Invalid panel specification".to_string()))
    })?;

    element_table.set("flex", flex_fn)?;
    element_table.set("grid", grid_fn)?;
    element_table.set("text", text_fn)?;
    element_table.set("progress", progress_fn)?;
    element_table.set("rect", rect_fn)?;
    element_table.set("image", image_fn)?;
    element_table.set("popup", popup_fn)?;
    element_table.set("panel", panel_fn)?;
    set_table_aliases!(element_table, module_fn, "module", "widget", "load_module");

    Ok(element_table)
}
