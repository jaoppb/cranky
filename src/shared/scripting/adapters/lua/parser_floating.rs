use super::vnode_parser::value_to_vnode;
use crate::features::vdom::domain::{
    AnchorDirection, ExclusiveZone, HorizontalAnchor, KeyboardInteractivity,
    PanelAnchor, PanelLayer, PanelSpec, PopupOffset, PopupSpec, VerticalAnchor,
};
use crate::shared::config::domain::{MarginConfig, MarginOffset};
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
            let t = m_table.get::<Option<i32>>("top").ok().flatten().unwrap_or(0);
            let r = m_table
                .get::<Option<i32>>("right")
                .ok()
                .flatten()
                .unwrap_or(0);
            let b = m_table
                .get::<Option<i32>>("bottom")
                .ok()
                .flatten()
                .unwrap_or(0);
            let l = m_table
                .get::<Option<i32>>("left")
                .ok()
                .flatten()
                .unwrap_or(0);
            MarginConfig::new(
                MarginOffset::new(t),
                MarginOffset::new(b),
                MarginOffset::new(l),
                MarginOffset::new(r),
            )
        },
    )
}

pub(crate) fn parse_popup_spec(
    lua: &Lua,
    val: Option<mlua::Value>,
) -> mlua::Result<Option<PopupSpec>> {
    let Some(val) = val else {
        return Ok(None);
    };
    match val {
        mlua::Value::UserData(ud) => {
            if let Ok(popup_ud) = ud.borrow::<super::userdata::LuaPopup>() {
                return Ok(Some(popup_ud.0.clone()));
            }
            if let Ok(vnode_ud) = ud.borrow::<super::userdata::LuaVNode>() {
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
                let dismiss_on_unfocus = table
                    .get::<Option<bool>>("dismiss_on_unfocus")
                    .ok()
                    .flatten()
                    .unwrap_or(true);
                Ok(Some(
                    PopupSpec::new(content)
                        .with_anchor(anchor)
                        .with_offset(offset)
                        .with_dismiss_on_unfocus(dismiss_on_unfocus),
                ))
            } else {
                let node = value_to_vnode(lua, mlua::Value::Table(table))?;
                Ok(Some(PopupSpec::new(Box::new(node))))
            }
        }
        _ => Ok(None),
    }
}

pub(crate) fn parse_panel_spec(
    lua: &Lua,
    val: Option<mlua::Value>,
) -> mlua::Result<Option<PanelSpec>> {
    let Some(val) = val else {
        return Ok(None);
    };
    match val {
        mlua::Value::UserData(ud) => {
            if let Ok(panel_ud) = ud.borrow::<super::userdata::LuaPanel>() {
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
            let mut top = false;
            let mut bottom = false;
            let mut left = false;
            let mut right = false;
            if let Ok(anchor_table) = table.get::<mlua::Table>("anchor") {
                for s in anchor_table.sequence_values::<String>().flatten() {
                    match s.to_lowercase().as_str() {
                        "top" => top = true,
                        "bottom" => bottom = true,
                        "left" => left = true,
                        "right" => right = true,
                        _ => {}
                    }
                }
            } else if let Ok(s) = table.get::<String>("anchor") {
                match s.to_lowercase().as_str() {
                    "top" => top = true,
                    "bottom" => bottom = true,
                    "left" => left = true,
                    "right" => right = true,
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
                    .with_anchor(PanelAnchor::new(
                        VerticalAnchor::from_edges(top, bottom),
                        HorizontalAnchor::from_edges(left, right),
                    ))
                    .with_margin(margin)
                    .with_exclusive_zone(exclusive_zone)
                    .with_keyboard(keyboard),
            ))
        }
        _ => Ok(None),
    }
}
