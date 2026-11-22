use super::vnode_parser::value_to_vnode;
use crate::features::vdom::domain::{
    AnchorDirection, ExclusiveZone, HorizontalAnchor, KeyboardInteractivity, PanelAnchor,
    PanelLayer, PanelSpec, PopupOffset, PopupSpec, VerticalAnchor,
};
use crate::shared::config::domain::{AxisMargins, MarginConfig, MarginOffset};
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

fn parse_margin(table: &mlua::Table) -> MarginConfig {
    table.get::<mlua::Table>("margin").map_or_else(
        |_| MarginConfig::default(),
        |m_table| {
            let top = m_table
                .get::<Option<i32>>("top")
                .ok()
                .flatten()
                .unwrap_or(0);
            let right = m_table
                .get::<Option<i32>>("right")
                .ok()
                .flatten()
                .unwrap_or(0);
            let bottom = m_table
                .get::<Option<i32>>("bottom")
                .ok()
                .flatten()
                .unwrap_or(0);
            let left = m_table
                .get::<Option<i32>>("left")
                .ok()
                .flatten()
                .unwrap_or(0);
            MarginConfig::new(
                AxisMargins::new(MarginOffset::new(top), MarginOffset::new(bottom)),
                AxisMargins::new(MarginOffset::new(left), MarginOffset::new(right)),
            )
        },
    )
}

pub(super) fn parse_popup_spec(
    lua: &Lua,
    val: Option<mlua::Value>,
) -> mlua::Result<Option<PopupSpec>> {
    let Some(val) = val else {
        return Ok(None);
    };
    match val {
        mlua::Value::UserData(user_data) => {
            if let Ok(popup_ud) = user_data.borrow::<super::userdata::LuaPopup>() {
                return Ok(Some(popup_ud.0.clone()));
            }
            if let Ok(vnode_ud) = user_data.borrow::<super::userdata::LuaVNode>() {
                return Ok(Some(PopupSpec::new(Box::new(vnode_ud.0.clone()))));
            }
            Ok(None)
        }
        mlua::Value::Table(table) => {
            if let Ok(content_val) = table
                .get::<mlua::Value>("content")
                .or_else(|_| table.get::<mlua::Value>("node"))
            {
                let content = Box::new(value_to_vnode(lua, content_val)?);
                let anchor_str: Option<String> = table.get("anchor").ok();
                let anchor = match anchor_str.as_deref() {
                    Some("top") => AnchorDirection::Top,
                    Some("bottom") => AnchorDirection::Bottom,
                    Some("left") => AnchorDirection::Left,
                    Some("right") => AnchorDirection::Right,
                    _ => AnchorDirection::Auto,
                };
                let offset = parse_offset(&table);
                Ok(Some(
                    PopupSpec::new(content)
                        .with_anchor(anchor)
                        .with_offset(offset),
                ))
            } else {
                let node = value_to_vnode(lua, mlua::Value::Table(table))?;
                Ok(Some(PopupSpec::new(Box::new(node))))
            }
        }
        _ => Ok(None),
    }
}

pub(super) fn parse_panel_spec(
    lua: &Lua,
    val: Option<mlua::Value>,
) -> mlua::Result<Option<PanelSpec>> {
    let Some(val) = val else {
        return Ok(None);
    };
    match val {
        mlua::Value::UserData(user_data) => {
            if let Ok(panel_ud) = user_data.borrow::<super::userdata::LuaPanel>() {
                return Ok(Some(panel_ud.0.clone()));
            }
            Ok(None)
        }
        mlua::Value::Table(table) => {
            let content_val = table
                .get::<mlua::Value>("content")
                .or_else(|_| table.get::<mlua::Value>("node"))?;
            let content = Box::new(value_to_vnode(lua, content_val)?);
            let layer_str: Option<String> = table.get("layer").ok();
            let layer = match layer_str.as_deref() {
                Some("background") => PanelLayer::Background,
                Some("bottom") => PanelLayer::Bottom,
                Some("overlay") => PanelLayer::Overlay,
                _ => PanelLayer::Top,
            };
            let mut vertical = VerticalAnchor::None;
            let mut horizontal = HorizontalAnchor::None;
            if let Ok(anchor_table) = table.get::<mlua::Table>("anchor") {
                for edge in anchor_table.sequence_values::<String>().flatten() {
                    match edge.to_lowercase().as_str() {
                        "top" => vertical = vertical.with_top(),
                        "bottom" => vertical = vertical.with_bottom(),
                        "left" => horizontal = horizontal.with_left(),
                        "right" => horizontal = horizontal.with_right(),
                        _ => {}
                    }
                }
            } else if let Ok(anchor) = table.get::<String>("anchor") {
                match anchor.to_lowercase().as_str() {
                    "top" => vertical = vertical.with_top(),
                    "bottom" => vertical = vertical.with_bottom(),
                    "left" => horizontal = horizontal.with_left(),
                    "right" => horizontal = horizontal.with_right(),
                    _ => {}
                }
            }
            let margin = parse_margin(&table);
            let exclusive_zone_val = table
                .get::<Option<i32>>("exclusive_zone")
                .ok()
                .flatten()
                .unwrap_or(0);
            let exclusive_zone = ExclusiveZone::new(exclusive_zone_val);
            let keyboard_str: Option<String> = table.get("keyboard").ok();
            let keyboard = match keyboard_str.as_deref() {
                Some("exclusive") => KeyboardInteractivity::Exclusive,
                Some("on_demand") => KeyboardInteractivity::OnDemand,
                _ => KeyboardInteractivity::None,
            };
            Ok(Some(
                PanelSpec::new(content)
                    .with_layer(layer)
                    .with_anchor(PanelAnchor::new(vertical, horizontal))
                    .with_margin(margin)
                    .with_exclusive_zone(exclusive_zone)
                    .with_keyboard(keyboard),
            ))
        }
        _ => Ok(None),
    }
}
