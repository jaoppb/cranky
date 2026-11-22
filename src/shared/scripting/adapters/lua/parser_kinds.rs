//! Reads the fields particular to one kind of Lua element table: a progress bar's value and
//! orientation, and an embedded module's name, instance and options.
use super::parser_props::parse_module_options;
use crate::features::styling::domain::{Orientation, ProgressValue};
use crate::features::vdom::domain::{ModuleParams, ProgressBar};
use crate::shared::primitives::{ModuleInstanceId, ModuleName};
use mlua::Lua;

/// The progress bar `table` describes: its `value`, empty when unset, along its `orientation`.
pub(super) fn parse_progress_bar(table: &mlua::Table) -> mlua::Result<ProgressBar> {
    let value_num = table.get::<Option<f32>>("value")?.unwrap_or(0.0);
    let orientation_str = table
        .get::<Option<String>>("orientation")?
        .unwrap_or_default();
    Ok(ProgressBar::new(
        ProgressValue::new(value_num).unwrap_or_default(),
        orientation_named(&orientation_str),
    ))
}

/// The orientation `name` names in any case: vertical for `"vertical"`, horizontal otherwise.
pub(super) fn orientation_named(name: &str) -> Orientation {
    match name.to_lowercase().as_str() {
        "vertical" => Orientation::Vertical,
        _ => Orientation::Horizontal,
    }
}

/// The module `table` embeds: its `name`, which it must have, its `instance_id` and `options`.
pub(super) fn parse_module_params(lua: &Lua, table: &mlua::Table) -> mlua::Result<ModuleParams> {
    let name_str = table.get::<String>("name")?;
    let instance_id_str = table.get::<Option<String>>("instance_id")?;
    let options = parse_module_options(lua, table)?;
    Ok(ModuleParams::new(
        ModuleName::new(name_str),
        instance_id_str.map(ModuleInstanceId::new),
        options,
    ))
}
