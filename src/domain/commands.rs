#[derive(Debug)]
pub enum AppCommand {
    CreateBar(u32, String),
    DestroyBar(u32),
    RequestRender(u32),
    Log(tracing::Level, String),
}
