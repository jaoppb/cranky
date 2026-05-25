#[derive(Debug)]
pub enum AppCommand {
    CreateBar(u32, String),
    DestroyBar(u32),
    RequestRender(u32),
    Input(crate::domain::ModuleId, crate::domain::events::InputEvent),
    Log(tracing::Level, String),
}
