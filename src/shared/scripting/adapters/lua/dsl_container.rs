use super::parser_props::{parse_common_props, parse_module_options};
use super::vnode_parser::{parse_table_children, value_to_vnode};
use crate::features::styling::domain::ClassNameList;
use crate::features::vdom::domain::{ElementSelectors, Interactions, VNode};
use crate::shared::primitives::{ModuleInstanceId, ModuleName, ModuleOptions};
use mlua::Lua;

pub(super) fn parse_flex_node(lua: &Lua, val: mlua::Value) -> mlua::Result<VNode> {
    let mlua::Value::Table(table) = val else {
        return Ok(VNode::new_flex(
            Vec::new(),
            ElementSelectors::default(),
            Interactions::default(),
        ));
    };

    let is_full_spec = table.contains_key("type")?
        || table.contains_key("class")?
        || table.contains_key("children")?
        || table.contains_key("id")?
        || table.contains_key("key")?
        || table.contains_key("on_click")?
        || table.contains_key("on_hover")?
        || table.contains_key("tooltip")?
        || table.contains_key("popup")?
        || table.contains_key("panel")?;

    if is_full_spec {
        let children_vec = parse_table_children(lua, &table)?;
        let (selectors, interactions, attachments) = parse_common_props(lua, &table)?;
        let node = VNode::new_flex(children_vec, selectors, interactions);
        return Ok(attachments.attach_to(node));
    }

    let mut children_vec = Vec::new();
    for pair in table.sequence_values::<mlua::Value>() {
        children_vec.push(value_to_vnode(lua, pair?)?);
    }
    Ok(VNode::new_flex(
        children_vec,
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

    let is_full_spec = table.contains_key("type")?
        || table.contains_key("class")?
        || table.contains_key("children")?
        || table.contains_key("id")?
        || table.contains_key("key")?
        || table.contains_key("on_click")?
        || table.contains_key("on_hover")?
        || table.contains_key("tooltip")?
        || table.contains_key("popup")?
        || table.contains_key("panel")?;

    if is_full_spec {
        let children_vec = parse_table_children(lua, &table)?;
        let (selectors, interactions, attachments) = parse_common_props(lua, &table)?;
        let node = VNode::new_grid(children_vec, selectors, interactions);
        return Ok(attachments.attach_to(node));
    }

    let mut children_vec = Vec::new();
    for pair in table.sequence_values::<mlua::Value>() {
        children_vec.push(value_to_vnode(lua, pair?)?);
    }
    Ok(VNode::new_grid(
        children_vec,
        ElementSelectors::default(),
        Interactions::default(),
    ))
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
            let name_str = table.get::<String>("name")?;
            let instance_id_str = table.get::<Option<String>>("instance_id")?;
            let options = parse_module_options(lua, &table)?;
            let (selectors, interactions, attachments) = parse_common_props(lua, &table)?;
            let params = crate::features::vdom::domain::ModuleParams::new(
                ModuleName::new(name_str),
                instance_id_str.map(ModuleInstanceId::new),
                options,
            );
            let node = VNode::new_module(params, selectors, interactions);
            Ok(attachments.attach_to(node))
        }
        mlua::Value::String(lua_string) => {
            let name_str = lua_string
                .to_str()
                .map_or_else(|_| String::new(), |text| text.to_string());
            let params = crate::features::vdom::domain::ModuleParams::new(
                ModuleName::new(name_str),
                None,
                ModuleOptions::default(),
            );
            Ok(VNode::new_module(
                params,
                ElementSelectors::default(),
                Interactions::default(),
            ))
        }
        _ => {
            let params = crate::features::vdom::domain::ModuleParams::new(
                ModuleName::new(String::new()),
                None,
                ModuleOptions::default(),
            );
            Ok(VNode::new_module(
                params,
                ElementSelectors::default(),
                Interactions::default(),
            ))
        }
    }
}
