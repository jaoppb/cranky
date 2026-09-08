use super::parser_props::{parse_common_props, parse_module_options};
use super::vnode_parser::{parse_table_children, value_to_vnode};
use crate::features::styling::domain::ClassNameList;
use crate::features::vdom::domain::VNode;
use crate::shared::primitives::{ModuleInstanceId, ModuleName, ModuleOptions};
use mlua::Lua;

pub(crate) fn parse_flex_node(lua: &Lua, val: mlua::Value) -> mlua::Result<VNode> {
    let mlua::Value::Table(table) = val else {
        return Ok(VNode::new_flex(Vec::new(), None, None, None, None, None));
    };

    let is_full_spec = table.contains_key("type")?
        || table.contains_key("class")?
        || table.contains_key("children")?
        || table.contains_key("id")?
        || table.contains_key("on_click")?
        || table.contains_key("on_hover")?
        || table.contains_key("tooltip")?
        || table.contains_key("popup")?
        || table.contains_key("panel")?;

    if is_full_spec {
        let children_vec = parse_table_children(lua, &table)?;
        let (class, id, on_click, on_hover, tooltip, popup, panel) =
            parse_common_props(lua, &table)?;
        let mut node = VNode::new_flex(children_vec, class, id, on_click, on_hover, tooltip);
        if let Some(p) = popup {
            node = node.with_popup(p);
        }
        if let Some(p) = panel {
            node = node.with_panel(p);
        }
        return Ok(node);
    }

    let mut children_vec = Vec::new();
    for pair in table.sequence_values::<mlua::Value>() {
        children_vec.push(value_to_vnode(lua, pair?)?);
    }
    Ok(VNode::new_flex(children_vec, None, None, None, None, None))
}

pub(crate) fn parse_grid_node(lua: &Lua, val: mlua::Value) -> mlua::Result<VNode> {
    let mlua::Value::Table(table) = val else {
        return Ok(VNode::new_grid(Vec::new(), None, None, None, None, None));
    };

    let is_full_spec = table.contains_key("type")?
        || table.contains_key("class")?
        || table.contains_key("children")?
        || table.contains_key("id")?
        || table.contains_key("on_click")?
        || table.contains_key("on_hover")?
        || table.contains_key("tooltip")?
        || table.contains_key("popup")?
        || table.contains_key("panel")?;

    if is_full_spec {
        let children_vec = parse_table_children(lua, &table)?;
        let (class, id, on_click, on_hover, tooltip, popup, panel) =
            parse_common_props(lua, &table)?;
        let mut node = VNode::new_grid(children_vec, class, id, on_click, on_hover, tooltip);
        if let Some(p) = popup {
            node = node.with_popup(p);
        }
        if let Some(p) = panel {
            node = node.with_panel(p);
        }
        return Ok(node);
    }

    let mut children_vec = Vec::new();
    for pair in table.sequence_values::<mlua::Value>() {
        children_vec.push(value_to_vnode(lua, pair?)?);
    }
    Ok(VNode::new_grid(children_vec, None, None, None, None, None))
}

pub(crate) fn parse_rect_node(lua: &Lua, val: Option<mlua::Value>) -> mlua::Result<VNode> {
    match val {
        Some(mlua::Value::Table(table)) => {
            let (class, id, on_click, on_hover, tooltip, popup, panel) =
                parse_common_props(lua, &table)?;
            let mut node = VNode::new_rect(class, id, on_click, on_hover, tooltip);
            if let Some(p) = popup {
                node = node.with_popup(p);
            }
            if let Some(p) = panel {
                node = node.with_panel(p);
            }
            Ok(node)
        }
        Some(mlua::Value::String(s)) => {
            let class = s
                .to_str()
                .ok()
                .and_then(|c| ClassNameList::parse(c.as_ref()).ok());
            Ok(VNode::new_rect(class, None, None, None, None))
        }
        _ => Ok(VNode::new_rect(None, None, None, None, None)),
    }
}

pub(crate) fn parse_module_node(lua: &Lua, val: mlua::Value) -> mlua::Result<VNode> {
    match val {
        mlua::Value::Table(table) => {
            let name_str = table.get::<String>("name")?;
            let instance_id_str = table.get::<Option<String>>("instance_id")?;
            let options = parse_module_options(lua, &table)?;
            let (class, id, on_click, on_hover, tooltip, popup, panel) =
                parse_common_props(lua, &table)?;
            let params = crate::features::vdom::domain::ModuleParams::new(
                ModuleName::new(name_str),
                instance_id_str.map(ModuleInstanceId::new),
                options,
            );
            let mut node = VNode::new_module(
                params,
                class,
                id,
                on_click,
                on_hover,
                tooltip,
            );
            if let Some(p) = popup {
                node = node.with_popup(p);
            }
            if let Some(p) = panel {
                node = node.with_panel(p);
            }
            Ok(node)
        }
        mlua::Value::String(s) => {
            let name_str = s.to_str().map_or_else(|_| String::new(), |b| b.to_string());
            let params = crate::features::vdom::domain::ModuleParams::new(
                ModuleName::new(name_str),
                None,
                ModuleOptions::default(),
            );
            Ok(VNode::new_module(
                params,
                None,
                None,
                None,
                None,
                None,
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
                None,
                None,
                None,
                None,
                None,
            ))
        }
    }
}
