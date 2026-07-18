use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum AppCommand {
    RequestRender,
    Exec(String),
    AppletAction {
        id: String,
        action: String,
    },
    ModuleSizeChanged(
        crate::domain::MonitorId,
        crate::domain::ModuleId,
        crate::domain::shared::geometry::Size,
    ),
    ShowTooltip {
        text: String,
    },
    HideTooltip,
}
