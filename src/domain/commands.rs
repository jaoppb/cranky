#[derive(Debug)]
pub enum AppCommand {
    RequestRender,
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
