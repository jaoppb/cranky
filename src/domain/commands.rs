#[derive(Debug)]
pub enum AppCommand {
    CreateBar(u32, String),
    DestroyBar(u32),
    RequestRender(u32),
    Log(tracing::Level, String),
    DBusCall(crate::domain::dbus::BusType, String, String, String, String, Vec<crate::domain::dbus::DBusValue>),
    AppletAction { id: String, action: String },
    ModuleSizeChanged(crate::domain::MonitorId, crate::domain::ModuleId, crate::domain::shared::geometry::Size),
    ShowTooltip { text: String },
    HideTooltip,
}
