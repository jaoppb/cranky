use super::panel::PanelSpec;
use super::popup::PopupSpec;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FloatingSpec {
    Popup(PopupSpec),
    Panel(PanelSpec),
}
