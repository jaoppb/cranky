use super::dsl_container::{parse_flex_node, parse_grid_node, parse_module_node, parse_rect_node};
use super::dsl_leaf::{parse_progress_table, parse_text_node, progress_from_value};
use super::parser_props::{
    parse_common_props, parse_image_data_and_size, parse_panel_spec, parse_popup_spec,
};
use super::userdata::{LuaPanel, LuaPopup, LuaVNode};
use crate::features::vdom::domain::{ImageContent, VNode};
use mlua::Lua;

macro_rules! set_table_aliases {
    ($table:expr, $value:expr, $($key:literal),*) => {
        $(
            $table.set($key, $value.clone())?;
        )*
    };
}

pub(super) fn create_ui_action_table(lua: &Lua) -> mlua::Result<mlua::Table> {
    let action_table = lua.create_table()?;
    action_table.set("exec", lua.create_function(exec_action)?)?;
    let call_fn = lua.create_function(script_call_action)?;
    set_table_aliases!(
        action_table,
        call_fn,
        "call",
        "script_call",
        "call_script",
        "func"
    );
    action_table.set("systray", lua.create_function(systray_action)?)?;
    Ok(action_table)
}

/// `ui.action.exec(cmd)`: the action running `cmd` as a shell command.
fn exec_action(lua: &Lua, cmd: String) -> mlua::Result<mlua::Table> {
    let table = lua.create_table()?;
    table.set("Exec", cmd)?;
    Ok(table)
}

/// `ui.action.call(name)`: the action calling the script's function `name`.
fn script_call_action(lua: &Lua, name: String) -> mlua::Result<mlua::Table> {
    let table = lua.create_table()?;
    table.set("ScriptCall", name)?;
    Ok(table)
}

/// `ui.action.systray(id, action)`: the action asking tray item `id` for `action`, `"Primary"`
/// when not given.
fn systray_action(
    lua: &Lua,
    (id, action): (mlua::Value, Option<String>),
) -> mlua::Result<mlua::Table> {
    let id_str = systray_item_id(&id);
    let action_str = action.unwrap_or_else(|| "Primary".to_string());
    let table = lua.create_table()?;
    let sub = lua.create_table()?;
    sub.set("id", id_str)?;
    sub.set("action", action_str)?;
    table.set("SystrayAction", sub)?;
    Ok(table)
}

/// The tray item id a script passed: a string, or a number written out; empty otherwise.
fn systray_item_id(id: &mlua::Value) -> String {
    match id {
        mlua::Value::String(lua_string) => lua_string
            .to_str()
            .map_or_else(|_| String::new(), |text| text.to_string()),
        mlua::Value::Integer(integer) => integer.to_string(),
        mlua::Value::Number(number) => number.to_string(),
        _ => String::new(),
    }
}

pub(super) fn create_ui_element_table(lua: &Lua) -> mlua::Result<mlua::Table> {
    let element_table = lua.create_table()?;
    element_table.set("flex", lua.create_function(flex_element)?)?;
    element_table.set("grid", lua.create_function(grid_element)?)?;
    element_table.set("text", lua.create_function(text_element)?)?;
    element_table.set("progress", lua.create_function(progress_element)?)?;
    element_table.set("rect", lua.create_function(rect_element)?)?;
    element_table.set(
        "image",
        lua.create_function(|lua, table: mlua::Table| image_element(lua, &table))?,
    )?;
    element_table.set("popup", lua.create_function(popup_element)?)?;
    element_table.set("panel", lua.create_function(panel_element)?)?;
    let module_fn = lua.create_function(module_element)?;
    set_table_aliases!(element_table, module_fn, "module", "widget", "load_module");
    Ok(element_table)
}

/// `ui.flex(...)`.
fn flex_element(lua: &Lua, val: mlua::Value) -> mlua::Result<LuaVNode> {
    Ok(LuaVNode(parse_flex_node(lua, val)?))
}

/// `ui.grid(...)`.
fn grid_element(lua: &Lua, val: mlua::Value) -> mlua::Result<LuaVNode> {
    Ok(LuaVNode(parse_grid_node(lua, val)?))
}

/// `ui.text(...)`.
fn text_element(
    lua: &Lua,
    (val, class_opt): (mlua::Value, Option<String>),
) -> mlua::Result<LuaVNode> {
    Ok(LuaVNode(parse_text_node(lua, val, class_opt.as_deref())?))
}

/// `ui.progress(...)`: a table describes the bar, any other value is how full it is.
fn progress_element(
    lua: &Lua,
    (val, orientation_opt, class_opt): (mlua::Value, Option<String>, Option<String>),
) -> mlua::Result<LuaVNode> {
    let node = match val {
        mlua::Value::Table(table) => parse_progress_table(lua, &table)?,
        other => progress_from_value(&other, orientation_opt.as_deref(), class_opt.as_deref()),
    };
    Ok(LuaVNode(node))
}

/// `ui.rect(...)`.
fn rect_element(lua: &Lua, val: Option<mlua::Value>) -> mlua::Result<LuaVNode> {
    Ok(LuaVNode(parse_rect_node(lua, val)?))
}

/// `ui.image({ data = ..., pixel_size = ..., ... })`: an image reacts to no click or hover.
fn image_element(lua: &Lua, table: &mlua::Table) -> mlua::Result<LuaVNode> {
    let (data, pixel_size) = parse_image_data_and_size(lua, table)?;
    let (selectors, interactions, attachments) = parse_common_props(lua, table)?;
    let node = VNode::new_image(
        ImageContent::new(data, pixel_size),
        selectors,
        interactions.into_tooltip(),
    );
    Ok(LuaVNode(attachments.attach_to(node)))
}

/// `ui.module(...)` and its aliases.
fn module_element(lua: &Lua, val: mlua::Value) -> mlua::Result<LuaVNode> {
    Ok(LuaVNode(parse_module_node(lua, val)?))
}

/// `ui.popup(...)`, an error for a value that describes no popup.
fn popup_element(lua: &Lua, val: mlua::Value) -> mlua::Result<LuaPopup> {
    let spec_opt = parse_popup_spec(lua, Some(val))?;
    spec_opt
        .map(LuaPopup)
        .ok_or_else(|| mlua::Error::RuntimeError("Invalid popup specification".to_string()))
}

/// `ui.panel(...)`, an error for a value that describes no panel.
fn panel_element(lua: &Lua, val: mlua::Value) -> mlua::Result<LuaPanel> {
    let spec_opt = parse_panel_spec(lua, Some(val))?;
    spec_opt
        .map(LuaPanel)
        .ok_or_else(|| mlua::Error::RuntimeError("Invalid panel specification".to_string()))
}
