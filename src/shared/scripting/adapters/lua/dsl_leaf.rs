use super::parser_props::{parse_common_props, parse_text_content};
use crate::features::styling::domain::{ClassNameList, Orientation, ProgressValue};
use crate::features::vdom::domain::{
    ElementSelectors, Interactions, ProgressBar, TextContent, VNode,
};
use crate::utils::f64_to_f32;
use mlua::Lua;

pub(super) fn parse_text_node(
    lua: &Lua,
    val: mlua::Value,
    class_opt: Option<&str>,
) -> mlua::Result<VNode> {
    match val {
        mlua::Value::Table(table) => {
            let text_str = parse_text_content(&table);
            let (selectors, interactions, attachments) = parse_common_props(lua, &table)?;
            let node = VNode::new_text(TextContent::new(text_str), selectors, interactions);
            Ok(attachments.attach_to(node))
        }
        mlua::Value::String(lua_string) => {
            let text_str = lua_string
                .to_str()
                .map_or_else(|_| String::new(), |text| text.to_string());
            let class = class_opt.and_then(|class_name| ClassNameList::parse(class_name).ok());
            Ok(VNode::new_text(
                TextContent::new(text_str),
                ElementSelectors::new(class, None),
                Interactions::default(),
            ))
        }
        mlua::Value::Integer(integer) => {
            let class = class_opt.and_then(|class_name| ClassNameList::parse(class_name).ok());
            Ok(VNode::new_text(
                TextContent::new(integer.to_string()),
                ElementSelectors::new(class, None),
                Interactions::default(),
            ))
        }
        mlua::Value::Number(number) => {
            let class = class_opt.and_then(|class_name| ClassNameList::parse(class_name).ok());
            Ok(VNode::new_text(
                TextContent::new(number.to_string()),
                ElementSelectors::new(class, None),
                Interactions::default(),
            ))
        }
        _ => Ok(VNode::new_text(
            TextContent::new(String::new()),
            ElementSelectors::default(),
            Interactions::default(),
        )),
    }
}

/// The progress node of `ui.progress({ value = ..., orientation = ..., ... })`.
pub(super) fn parse_progress_table(lua: &Lua, table: &mlua::Table) -> mlua::Result<VNode> {
    let value_num = table.get::<Option<f32>>("value")?.unwrap_or(0.0);
    let orientation_str = table
        .get::<Option<String>>("orientation")?
        .unwrap_or_default();
    let orientation = match orientation_str.to_lowercase().as_str() {
        "vertical" => Orientation::Vertical,
        _ => Orientation::Horizontal,
    };
    let (selectors, interactions, attachments) = parse_common_props(lua, table)?;
    let node = VNode::new_progress(
        ProgressBar::new(
            ProgressValue::new(value_num).unwrap_or_default(),
            orientation,
        ),
        selectors,
        interactions,
    );
    Ok(attachments.attach_to(node))
}

/// The progress node of `ui.progress(value, orientation, class)`: a number fills that far in
/// the given orientation with the given class, anything else is an empty horizontal bar.
pub(super) fn progress_from_value(
    val: &mlua::Value,
    orientation_opt: Option<&str>,
    class_opt: Option<&str>,
) -> VNode {
    let fill = match val {
        mlua::Value::Number(number) => Some(f64_to_f32(*number)),
        mlua::Value::Integer(integer) => {
            Some(f64_to_f32(i32::try_from(*integer).map_or(0.0, f64::from)))
        }
        _ => None,
    };
    let Some(val_f32) = fill else {
        return VNode::new_progress(
            ProgressBar::new(ProgressValue::default(), Orientation::Horizontal),
            ElementSelectors::default(),
            Interactions::default(),
        );
    };
    let orientation = match orientation_opt
        .unwrap_or("horizontal")
        .to_lowercase()
        .as_str()
    {
        "vertical" => Orientation::Vertical,
        _ => Orientation::Horizontal,
    };
    let class = class_opt.and_then(|class_name| ClassNameList::parse(class_name).ok());
    VNode::new_progress(
        ProgressBar::new(ProgressValue::new(val_f32).unwrap_or_default(), orientation),
        ElementSelectors::new(class, None),
        Interactions::default(),
    )
}
