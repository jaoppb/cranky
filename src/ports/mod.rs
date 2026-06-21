pub mod canvas;
pub mod dbus;
pub mod font;
pub mod sni;

pub use dbus::DBusPort;
pub mod registry;
pub mod surface;

use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DisplayServerError {
    #[error("Display server connection failed: {reason}")]
    ConnectionFailed { reason: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Error, Debug)]
pub enum WindowManagerError {
    #[error("Window manager IPC error: {reason}")]
    IpcError { reason: String },
}

#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait DisplayServerPort: Send + Sync {
    async fn wait_for_events(&mut self) -> Result<(), DisplayServerError>;
    fn dispatch_pending(&mut self) -> Result<(), DisplayServerError>;
    fn flush(&mut self) -> Result<(), DisplayServerError>;
    fn render_all(
        &mut self,
        read_model: &crate::domain::app::AppReadModel,
        layout_senders: &std::collections::HashMap<
            crate::domain::ModuleId,
            Box<dyn crate::ports::registry::LayoutSender>,
        >,
    ) -> Result<(), DisplayServerError>;
    fn show_tooltip(&mut self, text: &str) -> Result<(), DisplayServerError>;
    fn hide_tooltip(&mut self) -> Result<(), DisplayServerError>;
}

pub trait WindowManagerPort: Send + Sync {
    fn get_state(
        &self,
    ) -> Result<
        (
            Vec<crate::domain::workspace::Workspace>,
            Vec<crate::domain::workspace::Monitor>,
        ),
        WindowManagerError,
    >;
}
