//! Reads the panel a Lua element opens: its content, layer, anchor edges, margin, exclusive
//! zone and keyboard mode.
use super::userdata::LuaPanel;
use super::vnode_parser::value_to_vnode;
use crate::features::vdom::domain::{
    ExclusiveZone, HorizontalAnchor, KeyboardInteractivity, PanelAnchor, PanelLayer, PanelSpec,
    VerticalAnchor,
};
use crate::shared::config::domain::{AxisMargins, MarginConfig, MarginOffset};
use mlua::Lua;

fn parse_margin(table: &mlua::Table) -> MarginConfig {
    table.get::<mlua::Table>("margin").map_or_else(
        |_| MarginConfig::default(),
        |margin_table| {
            let top = margin_side(&margin_table, "top");
            let right = margin_side(&margin_table, "right");
            let bottom = margin_side(&margin_table, "bottom");
            let left = margin_side(&margin_table, "left");
            MarginConfig::new(
                AxisMargins::new(MarginOffset::new(top), MarginOffset::new(bottom)),
                AxisMargins::new(MarginOffset::new(left), MarginOffset::new(right)),
            )
        },
    )
}

/// The margin `margin_table` gives the side `side`, zero when it gives none.
fn margin_side(margin_table: &mlua::Table, side: &str) -> i32 {
    margin_table
        .get::<Option<i32>>(side)
        .ok()
        .flatten()
        .unwrap_or(0)
}

pub(super) fn parse_panel_spec(
    lua: &Lua,
    val: Option<mlua::Value>,
) -> mlua::Result<Option<PanelSpec>> {
    let Some(val) = val else {
        return Ok(None);
    };
    match val {
        mlua::Value::UserData(user_data) => Ok(user_data
            .borrow::<LuaPanel>()
            .ok()
            .map(|panel_ud| panel_ud.0.clone())),
        mlua::Value::Table(table) => parse_panel_table(lua, &table).map(Some),
        _ => Ok(None),
    }
}

/// The panel `table` describes: its `content` (or `node` when reading `content` fails), `layer`,
/// `anchor`, `margin`, `exclusive_zone` and `keyboard`.
fn parse_panel_table(lua: &Lua, table: &mlua::Table) -> mlua::Result<PanelSpec> {
    let content_val = table
        .get::<mlua::Value>("content")
        .or_else(|_| table.get::<mlua::Value>("node"))?;
    let content = Box::new(value_to_vnode(lua, content_val)?);
    let layer_str: Option<String> = table.get("layer").ok();
    let layer = panel_layer_named(layer_str.as_deref());
    let anchor = parse_panel_anchor(table);
    let margin = parse_margin(table);
    let exclusive_zone_val = table
        .get::<Option<i32>>("exclusive_zone")
        .ok()
        .flatten()
        .unwrap_or(0);
    let keyboard_str: Option<String> = table.get("keyboard").ok();
    Ok(PanelSpec::new(content)
        .with_layer(layer)
        .with_anchor(anchor)
        .with_margin(margin)
        .with_exclusive_zone(ExclusiveZone::new(exclusive_zone_val))
        .with_keyboard(keyboard_named(keyboard_str.as_deref())))
}

/// The layer `name` names; `Top` for any other name or none.
fn panel_layer_named(name: Option<&str>) -> PanelLayer {
    match name {
        Some("background") => PanelLayer::Background,
        Some("bottom") => PanelLayer::Bottom,
        Some("overlay") => PanelLayer::Overlay,
        _ => PanelLayer::Top,
    }
}

/// The keyboard mode `name` names; `None` for any other name or none.
fn keyboard_named(name: Option<&str>) -> KeyboardInteractivity {
    match name {
        Some("exclusive") => KeyboardInteractivity::Exclusive,
        Some("on_demand") => KeyboardInteractivity::OnDemand,
        _ => KeyboardInteractivity::None,
    }
}

/// The edges the `anchor` of `table` names, as a list or as one string.
fn parse_panel_anchor(table: &mlua::Table) -> PanelAnchor {
    let edges: Vec<String> = table.get::<mlua::Table>("anchor").map_or_else(
        |_| table.get::<String>("anchor").into_iter().collect(),
        |anchor_table| anchor_table.sequence_values::<String>().flatten().collect(),
    );
    let (vertical, horizontal) = edges.iter().fold(
        (VerticalAnchor::None, HorizontalAnchor::None),
        |anchor, edge| with_anchor_edge(anchor, edge),
    );
    PanelAnchor::new(vertical, horizontal)
}

/// The vertical and horizontal anchors also anchored to the edge `edge` names in any case;
/// unchanged for any other name.
fn with_anchor_edge(
    (vertical, horizontal): (VerticalAnchor, HorizontalAnchor),
    edge: &str,
) -> (VerticalAnchor, HorizontalAnchor) {
    match edge.to_lowercase().as_str() {
        "top" => (vertical.with_top(), horizontal),
        "bottom" => (vertical.with_bottom(), horizontal),
        "left" => (vertical, horizontal.with_left()),
        "right" => (vertical, horizontal.with_right()),
        _ => (vertical, horizontal),
    }
}
