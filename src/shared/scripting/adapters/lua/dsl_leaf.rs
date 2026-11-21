use super::parser_props::{parse_common_props, parse_text_content};
use crate::features::styling::domain::{ClassNameList, Orientation, ProgressValue};
use crate::features::vdom::domain::{TextContent, VNode};
use crate::utils::f64_to_f32;
use mlua::Lua;

pub(crate) fn parse_text_node(
    lua: &Lua,
    val: mlua::Value,
    class_opt: Option<&str>,
) -> mlua::Result<VNode> {
    match val {
        mlua::Value::Table(table) => {
            let text_str = parse_text_content(&table);
            let (class, id, on_click, on_hover, tooltip, popup, panel, key) =
                parse_common_props(lua, &table)?;
            let mut node = VNode::new_text(
                TextContent::new(text_str),
                class,
                id,
                on_click,
                on_hover,
                tooltip,
            );
            if let Some(popup_spec) = popup {
                node = node.with_popup(popup_spec);
            }
            if let Some(panel_spec) = panel {
                node = node.with_panel(panel_spec);
            }
            if let Some(node_key) = key {
                node = node.with_key(node_key);
            }
            Ok(node)
        }
        mlua::Value::String(lua_string) => {
            let text_str = lua_string
                .to_str()
                .map_or_else(|_| String::new(), |text| text.to_string());
            let class = class_opt.and_then(|class_name| ClassNameList::parse(class_name).ok());
            Ok(VNode::new_text(
                TextContent::new(text_str),
                class,
                None,
                None,
                None,
                None,
            ))
        }
        mlua::Value::Integer(integer) => {
            let class = class_opt.and_then(|class_name| ClassNameList::parse(class_name).ok());
            Ok(VNode::new_text(
                TextContent::new(integer.to_string()),
                class,
                None,
                None,
                None,
                None,
            ))
        }
        mlua::Value::Number(number) => {
            let class = class_opt.and_then(|class_name| ClassNameList::parse(class_name).ok());
            Ok(VNode::new_text(
                TextContent::new(number.to_string()),
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

pub(crate) fn parse_progress_node(
    lua: &Lua,
    val: mlua::Value,
    orientation_opt: Option<&str>,
    class_opt: Option<&str>,
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
            let (class, id, on_click, on_hover, tooltip, popup, panel, key) =
                parse_common_props(lua, &table)?;
            let mut node = VNode::new_progress(
                ProgressValue::new(value_num).unwrap_or_default(),
                orientation,
                class,
                id,
                on_click,
                on_hover,
                tooltip,
            );
            if let Some(popup_spec) = popup {
                node = node.with_popup(popup_spec);
            }
            if let Some(panel_spec) = panel {
                node = node.with_panel(panel_spec);
            }
            if let Some(node_key) = key {
                node = node.with_key(node_key);
            }
            Ok(node)
        }
        mlua::Value::Number(number) => {
            let val_f32 = f64_to_f32(number);
            let orientation = match orientation_opt
                .unwrap_or("horizontal")
                .to_lowercase()
                .as_str()
            {
                "vertical" => Orientation::Vertical,
                _ => Orientation::Horizontal,
            };
            let class = class_opt.and_then(|class_name| ClassNameList::parse(class_name).ok());
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
        mlua::Value::Integer(integer) => {
            let val_f32 = f64_to_f32(i32::try_from(integer).map_or(0.0, f64::from));
            let orientation = match orientation_opt
                .unwrap_or("horizontal")
                .to_lowercase()
                .as_str()
            {
                "vertical" => Orientation::Vertical,
                _ => Orientation::Horizontal,
            };
            let class = class_opt.and_then(|class_name| ClassNameList::parse(class_name).ok());
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
