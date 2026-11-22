use super::userdata::{LuaPopup, LuaVNode};
use super::vnode_parser::value_to_vnode;
use crate::features::vdom::domain::{AnchorDirection, PopupOffset, PopupSpec};
use mlua::Lua;

fn parse_offset(table: &mlua::Table) -> Option<PopupOffset> {
    table.get::<mlua::Table>("offset").ok().map(|offset_table| {
        let dx = offset_table
            .get::<Option<i32>>("x")
            .ok()
            .flatten()
            .or_else(|| offset_table.get::<Option<i32>>("dx").ok().flatten())
            .unwrap_or(0);
        let dy = offset_table
            .get::<Option<i32>>("y")
            .ok()
            .flatten()
            .or_else(|| offset_table.get::<Option<i32>>("dy").ok().flatten())
            .unwrap_or(0);
        PopupOffset::new(dx, dy)
    })
}

pub(super) fn parse_popup_spec(
    lua: &Lua,
    val: Option<mlua::Value>,
) -> mlua::Result<Option<PopupSpec>> {
    let Some(val) = val else {
        return Ok(None);
    };
    match val {
        mlua::Value::UserData(user_data) => Ok(popup_from_userdata(&user_data)),
        mlua::Value::Table(table) => parse_popup_table(lua, table).map(Some),
        _ => Ok(None),
    }
}

/// The popup a `ui.popup` value stands for, or a popup showing a node built with `ui.*`; none
/// for other userdata.
fn popup_from_userdata(user_data: &mlua::AnyUserData) -> Option<PopupSpec> {
    if let Ok(popup_ud) = user_data.borrow::<LuaPopup>() {
        return Some(popup_ud.0.clone());
    }
    let vnode_ud = user_data.borrow::<LuaVNode>().ok()?;
    Some(PopupSpec::new(Box::new(vnode_ud.0.clone())))
}

/// The popup `table` describes: its `content`, with its `anchor` and `offset`. A missing
/// `content` reads as nil, which is no node and so an error; `node` is read instead only when
/// reading `content` itself errors, and when that errors too the table itself is the content.
fn parse_popup_table(lua: &Lua, table: mlua::Table) -> mlua::Result<PopupSpec> {
    let Ok(content_val) = table
        .get::<mlua::Value>("content")
        .or_else(|_| table.get::<mlua::Value>("node"))
    else {
        let node = value_to_vnode(lua, mlua::Value::Table(table))?;
        return Ok(PopupSpec::new(Box::new(node)));
    };
    let content = Box::new(value_to_vnode(lua, content_val)?);
    let anchor_str: Option<String> = table.get("anchor").ok();
    let anchor = anchor_direction_named(anchor_str.as_deref());
    let offset = parse_offset(&table);
    Ok(PopupSpec::new(content)
        .with_anchor(anchor)
        .with_offset(offset))
}

/// The side `name` names, case-sensitively; `Auto` for any other name or none.
fn anchor_direction_named(name: Option<&str>) -> AnchorDirection {
    match name {
        Some("top") => AnchorDirection::Top,
        Some("bottom") => AnchorDirection::Bottom,
        Some("left") => AnchorDirection::Left,
        Some("right") => AnchorDirection::Right,
        _ => AnchorDirection::Auto,
    }
}
