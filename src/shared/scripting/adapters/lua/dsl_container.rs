use super::parser_kinds::parse_module_params;
use super::parser_props::parse_common_props;
use super::vnode_parser::{parse_table_children, value_to_vnode};
use crate::features::styling::domain::ClassNameList;
use crate::features::vdom::domain::{ElementSelectors, Interactions, ModuleParams, VNode};
use crate::shared::primitives::{ModuleName, ModuleOptions};
use mlua::Lua;

/// The keys that make a container table a full element specification, whose children are under
/// `children`, rather than a shorthand sequence of children.
const FULL_SPEC_KEYS: [&str; 10] = [
    "type", "class", "children", "id", "key", "on_click", "on_hover", "tooltip", "popup", "panel",
];

pub(super) fn parse_flex_node(lua: &Lua, val: mlua::Value) -> mlua::Result<VNode> {
    let mlua::Value::Table(table) = val else {
        return Ok(VNode::new_flex(
            Vec::new(),
            ElementSelectors::default(),
            Interactions::default(),
        ));
    };

    if is_full_spec(&table)? {
        let children_vec = parse_table_children(lua, &table)?;
        let (selectors, interactions, attachments) = parse_common_props(lua, &table)?;
        let node = VNode::new_flex(children_vec, selectors, interactions);
        return Ok(attachments.attach_to(node));
    }

    Ok(VNode::new_flex(
        parse_sequence_children(lua, &table)?,
        ElementSelectors::default(),
        Interactions::default(),
    ))
}

pub(super) fn parse_grid_node(lua: &Lua, val: mlua::Value) -> mlua::Result<VNode> {
    let mlua::Value::Table(table) = val else {
        return Ok(VNode::new_grid(
            Vec::new(),
            ElementSelectors::default(),
            Interactions::default(),
        ));
    };

    if is_full_spec(&table)? {
        let children_vec = parse_table_children(lua, &table)?;
        let (selectors, interactions, attachments) = parse_common_props(lua, &table)?;
        let node = VNode::new_grid(children_vec, selectors, interactions);
        return Ok(attachments.attach_to(node));
    }

    Ok(VNode::new_grid(
        parse_sequence_children(lua, &table)?,
        ElementSelectors::default(),
        Interactions::default(),
    ))
}

/// Whether `table` has any of the keys of a full element specification.
fn is_full_spec(table: &mlua::Table) -> mlua::Result<bool> {
    for key in FULL_SPEC_KEYS {
        if table.contains_key(key)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The nodes of the sequence part of `table`, in order.
fn parse_sequence_children(lua: &Lua, table: &mlua::Table) -> mlua::Result<Vec<VNode>> {
    let mut children_vec = Vec::new();
    for pair in table.sequence_values::<mlua::Value>() {
        children_vec.push(value_to_vnode(lua, pair?)?);
    }
    Ok(children_vec)
}

pub(super) fn parse_rect_node(lua: &Lua, val: Option<mlua::Value>) -> mlua::Result<VNode> {
    match val {
        Some(mlua::Value::Table(table)) => {
            let (selectors, interactions, attachments) = parse_common_props(lua, &table)?;
            let node = VNode::new_rect(selectors, interactions);
            Ok(attachments.attach_to(node))
        }
        Some(mlua::Value::String(lua_string)) => {
            let class = lua_string
                .to_str()
                .ok()
                .and_then(|text| ClassNameList::parse(text.as_ref()).ok());
            Ok(VNode::new_rect(
                ElementSelectors::new(class, None),
                Interactions::default(),
            ))
        }
        _ => Ok(VNode::new_rect(
            ElementSelectors::default(),
            Interactions::default(),
        )),
    }
}

pub(super) fn parse_module_node(lua: &Lua, val: mlua::Value) -> mlua::Result<VNode> {
    match val {
        mlua::Value::Table(table) => {
            let params = parse_module_params(lua, &table)?;
            let (selectors, interactions, attachments) = parse_common_props(lua, &table)?;
            let node = VNode::new_module(params, selectors, interactions);
            Ok(attachments.attach_to(node))
        }
        mlua::Value::String(lua_string) => {
            let name_str = lua_string
                .to_str()
                .map_or_else(|_| String::new(), |text| text.to_string());
            Ok(named_module_node(name_str))
        }
        _ => Ok(named_module_node(String::new())),
    }
}

/// A node embedding the module `name`, with no instance, options, selectors or interactions.
fn named_module_node(name: String) -> VNode {
    let params = ModuleParams::new(ModuleName::new(name), None, ModuleOptions::default());
    VNode::new_module(params, ElementSelectors::default(), Interactions::default())
}
