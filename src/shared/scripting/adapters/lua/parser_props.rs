pub(super) use super::parser_floating::{parse_panel_spec, parse_popup_spec};
use super::vnode_parser::value_to_vnode;
use crate::features::styling::domain::{ClassNameList, ElementId};
use crate::features::vdom::domain::{
    ClickHandlers, NodeKey, PanelSpec, PopupSpec, UiAction, VNode,
};
use crate::shared::events::core::PointerButton;
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::{BinaryData, ModuleOptions};
use mlua::{Lua, LuaSerdeExt};

fn parse_ui_action(lua: &Lua, val: Option<mlua::Value>) -> Option<UiAction> {
    match val {
        Some(mlua::Value::Table(table)) => {
            let cmd: Result<UiAction, _> = lua.from_value(mlua::Value::Table(table));
            cmd.ok()
        }
        _ => None,
    }
}

fn parse_pixel_size(table: &mlua::Table) -> mlua::Result<Size> {
    let Some(size_table) = table.get::<Option<mlua::Table>>("pixel_size")? else {
        return Ok(Size::new(0, 0));
    };
    let width = size_table.get::<Option<u32>>("width")?.unwrap_or(0);
    let height = size_table.get::<Option<u32>>("height")?.unwrap_or(0);
    Ok(Size::new(width, height))
}

fn parse_image_data(lua: &Lua, table: &mlua::Table) -> BinaryData {
    match table.get::<mlua::Value>("data") {
        Ok(mlua::Value::String(lua_string)) => BinaryData::new(lua_string.as_bytes().to_vec()),
        Ok(val) => lua
            .from_value(val)
            .unwrap_or_else(|_| BinaryData::new(vec![])),
        Err(_) => BinaryData::new(vec![]),
    }
}

pub(super) fn parse_image_data_and_size(
    lua: &Lua,
    table: &mlua::Table,
) -> mlua::Result<(BinaryData, Size)> {
    let size = parse_pixel_size(table)?;
    let data = parse_image_data(lua, table);
    Ok((data, size))
}

pub(super) fn parse_text_content(table: &mlua::Table) -> String {
    match table.get::<mlua::Value>("text") {
        Ok(mlua::Value::String(lua_string)) => lua_string
            .to_str()
            .map_or_else(|_| String::new(), |text| text.to_string()),
        Ok(mlua::Value::Integer(integer)) => integer.to_string(),
        Ok(mlua::Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

pub(super) fn parse_module_options(lua: &Lua, table: &mlua::Table) -> mlua::Result<ModuleOptions> {
    Ok(table
        .get::<Option<mlua::Value>>("options")?
        .and_then(|opts_val| lua.from_value(opts_val).ok())
        .unwrap_or_default())
}

fn parse_click_handlers(lua: &Lua, val: Option<mlua::Value>) -> Option<ClickHandlers> {
    let Some(mlua::Value::Table(table)) = val else {
        return None;
    };

    let mut handlers = ClickHandlers::new();

    for (key, value) in table.pairs::<mlua::Value, mlua::Value>().flatten() {
        let button_opt = match key {
            mlua::Value::String(lua_string) => lua_string
                .to_str()
                .ok()
                .as_deref()
                .and_then(PointerButton::from_name),
            mlua::Value::Integer(integer) => {
                u32::try_from(integer).ok().map(PointerButton::from_raw)
            }
            mlua::Value::Number(number) => {
                if number >= 0.0 && number.fract() == 0.0 && number <= f64::from(u32::MAX) {
                    Some(PointerButton::from_raw(crate::utils::f32_to_u32(
                        crate::utils::f64_to_f32(number),
                    )))
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(btn) = button_opt
            && let Some(cmd) = parse_ui_action(lua, Some(value))
        {
            handlers.insert(btn, cmd);
        }
    }

    if !handlers.is_empty() {
        return Some(handlers);
    }

    parse_ui_action(lua, Some(mlua::Value::Table(table))).map(ClickHandlers::from_single)
}

type CommonProps = (
    Option<ClassNameList>,
    Option<ElementId>,
    Option<ClickHandlers>,
    Option<UiAction>,
    Option<Box<VNode>>,
    Option<PopupSpec>,
    Option<PanelSpec>,
    Option<NodeKey>,
);

pub(super) fn parse_common_props(lua: &Lua, table: &mlua::Table) -> mlua::Result<CommonProps> {
    let class = table
        .get::<Option<String>>("class")?
        .and_then(|class_names| ClassNameList::parse(&class_names).ok());
    let id = table
        .get::<Option<String>>("id")?
        .and_then(|raw_id| ElementId::new(raw_id).ok());
    let on_click = parse_click_handlers(lua, table.get::<Option<mlua::Value>>("on_click")?);
    let on_hover = parse_ui_action(lua, table.get::<Option<mlua::Value>>("on_hover")?);
    let tooltip = match table.get::<Option<mlua::Value>>("tooltip")? {
        Some(mlua::Value::Nil) | None => None,
        Some(val) => Some(Box::new(value_to_vnode(lua, val)?)),
    };
    let popup = parse_popup_spec(lua, table.get::<Option<mlua::Value>>("popup")?)?;
    let panel = parse_panel_spec(lua, table.get::<Option<mlua::Value>>("panel")?)?;
    // Reconciliation identity, distinct from `id` (a CSS selector). A widget
    // rendering a variable-length list sets this so the layout and vdom
    // reconcilers can track a child across insertions/removals instead of by
    // position — see `NodeKey` for the "all siblings keyed, or none" policy.
    let key = table
        .get::<Option<String>>("key")?
        .and_then(|raw_key| NodeKey::new(raw_key).ok());
    Ok((class, id, on_click, on_hover, tooltip, popup, panel, key))
}
