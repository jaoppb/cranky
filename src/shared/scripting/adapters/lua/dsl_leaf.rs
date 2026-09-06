use super::parser_props::{parse_common_props, parse_text_content};
use crate::features::styling::domain::{ClassNameList, Orientation, ProgressValue};
use crate::features::vdom::domain::{TextContent, VNode};
use mlua::Lua;

pub(crate) fn parse_text_node(
    lua: &Lua,
    val: mlua::Value,
    class_opt: Option<&str>,
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
            let class = class_opt.and_then(|c| ClassNameList::parse(c).ok());
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
            let class = class_opt.and_then(|c| ClassNameList::parse(c).ok());
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
            let class = class_opt.and_then(|c| ClassNameList::parse(c).ok());
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
                .unwrap_or("horizontal")
                .to_lowercase()
                .as_str()
            {
                "vertical" => Orientation::Vertical,
                _ => Orientation::Horizontal,
            };
            let class = class_opt.and_then(|c| ClassNameList::parse(c).ok());
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
                .unwrap_or("horizontal")
                .to_lowercase()
                .as_str()
            {
                "vertical" => Orientation::Vertical,
                _ => Orientation::Horizontal,
            };
            let class = class_opt.and_then(|c| ClassNameList::parse(c).ok());
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
