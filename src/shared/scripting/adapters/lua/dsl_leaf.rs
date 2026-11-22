use super::parser_kinds::{orientation_named, parse_progress_bar};
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
            Ok(classed_text_node(text_str, class_opt))
        }
        mlua::Value::Integer(integer) => Ok(classed_text_node(integer.to_string(), class_opt)),
        mlua::Value::Number(number) => Ok(classed_text_node(number.to_string(), class_opt)),
        _ => Ok(VNode::new_text(
            TextContent::new(String::new()),
            ElementSelectors::default(),
            Interactions::default(),
        )),
    }
}

/// A text node showing `text`, with the class names `class_opt` unless they do not parse.
fn classed_text_node(text: String, class_opt: Option<&str>) -> VNode {
    let class = class_opt.and_then(|class_name| ClassNameList::parse(class_name).ok());
    VNode::new_text(
        TextContent::new(text),
        ElementSelectors::new(class, None),
        Interactions::default(),
    )
}

/// The progress node of `ui.progress({ value = ..., orientation = ..., ... })`.
pub(super) fn parse_progress_table(lua: &Lua, table: &mlua::Table) -> mlua::Result<VNode> {
    let progress = parse_progress_bar(table)?;
    let (selectors, interactions, attachments) = parse_common_props(lua, table)?;
    let node = VNode::new_progress(progress, selectors, interactions);
    Ok(attachments.attach_to(node))
}

/// The progress node of `ui.progress(value, orientation, class)`: a number fills that far in
/// the given orientation with the given class, anything else is an empty horizontal bar.
pub(super) fn progress_from_value(
    val: &mlua::Value,
    orientation_opt: Option<&str>,
    class_opt: Option<&str>,
) -> VNode {
    let Some(val_f32) = progress_fill(val) else {
        return VNode::new_progress(
            ProgressBar::new(ProgressValue::default(), Orientation::Horizontal),
            ElementSelectors::default(),
            Interactions::default(),
        );
    };
    let orientation = orientation_named(orientation_opt.unwrap_or("horizontal"));
    let class = class_opt.and_then(|class_name| ClassNameList::parse(class_name).ok());
    VNode::new_progress(
        ProgressBar::new(ProgressValue::new(val_f32).unwrap_or_default(), orientation),
        ElementSelectors::new(class, None),
        Interactions::default(),
    )
}

/// How far the value `val` fills a progress bar: a number as it is, an integer within `i32` as
/// that number and a larger one not at all; none for any other value.
fn progress_fill(val: &mlua::Value) -> Option<f32> {
    match val {
        mlua::Value::Number(number) => Some(f64_to_f32(*number)),
        mlua::Value::Integer(integer) => {
            Some(f64_to_f32(i32::try_from(*integer).map_or(0.0, f64::from)))
        }
        _ => None,
    }
}
