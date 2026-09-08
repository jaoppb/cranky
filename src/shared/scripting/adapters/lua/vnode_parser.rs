use super::parser_props::{
    parse_common_props, parse_image_data_and_size, parse_module_options, parse_text_content,
};
use super::userdata::LuaVNode;
use crate::features::styling::domain::{Orientation, ProgressValue};
use crate::features::vdom::domain::{TextContent, VNode};
use crate::shared::primitives::{ModuleInstanceId, ModuleName};
use mlua::{Lua, LuaSerdeExt};

pub(crate) fn parse_table_children(lua: &Lua, table: &mlua::Table) -> mlua::Result<Vec<VNode>> {
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
    let (class, id, on_click, on_hover, tooltip, popup, panel) =
        parse_common_props(lua, &table)?;

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
            let params = crate::features::vdom::domain::ModuleParams::new(
                ModuleName::new(name_str),
                instance_id_str.map(ModuleInstanceId::new),
                options,
            );
            VNode::new_module(
                params,
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
    if let Some(p) = panel {
        node = node.with_panel(p);
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
