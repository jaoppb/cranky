//! Turns the tables a Lua `render()` returns into a `VNode` tree.
use super::parser_kinds::{parse_module_params, parse_progress_bar};
use super::parser_props::{parse_common_props, parse_image_data_and_size, parse_text_content};
use super::userdata::LuaVNode;
use crate::features::vdom::domain::{ImageContent, TextContent, VNode};
use mlua::{Lua, LuaSerdeExt};

pub(super) fn parse_table_children(lua: &Lua, table: &mlua::Table) -> mlua::Result<Vec<VNode>> {
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
    let (selectors, interactions, attachments) = parse_common_props(lua, &table)?;

    let node = match typ.as_str() {
        "flex" | "" => VNode::new_flex(parse_table_children(lua, &table)?, selectors, interactions),
        "grid" => VNode::new_grid(parse_table_children(lua, &table)?, selectors, interactions),
        "text" => {
            let text_str = parse_text_content(&table);
            VNode::new_text(TextContent::new(text_str), selectors, interactions)
        }
        "progress" => VNode::new_progress(parse_progress_bar(&table)?, selectors, interactions),
        "rect" => VNode::new_rect(selectors, interactions),
        "image" => {
            let (data, pixel_size) = parse_image_data_and_size(lua, &table)?;
            let image = ImageContent::new(data, pixel_size);
            VNode::new_image(image, selectors, interactions.into_tooltip())
        }
        "module" => VNode::new_module(parse_module_params(lua, &table)?, selectors, interactions),
        _ => return lua.from_value::<VNode>(mlua::Value::Table(table)),
    };

    Ok(attachments.attach_to(node))
}

/// Converts a Lua value (either `LuaVNode` `UserData` or a table) into a `VNode`.
///
/// # Errors
///
/// Returns `mlua::Error` if value conversion fails.
pub(crate) fn value_to_vnode(lua: &Lua, val: mlua::Value) -> mlua::Result<VNode> {
    match val {
        mlua::Value::UserData(user_data) => {
            if let Ok(lua_vnode) = user_data.borrow::<LuaVNode>() {
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
