pub mod canvas;
pub mod font;
pub mod dbus;
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
    #[error("Surface error for target {target_id}: {reason}")]
    SurfaceError { target_id: u32, reason: String },
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
    fn create_bar(&self, output_id: u32, name: &str) -> Result<(), DisplayServerError>;
    fn destroy_bar(&self, output_id: u32) -> Result<(), DisplayServerError>;
    async fn wait_for_events(&mut self) -> Result<(), DisplayServerError>;
    fn dispatch_pending(&mut self) -> Result<(), DisplayServerError>;
    fn flush(&mut self) -> Result<(), DisplayServerError>;
    fn render_all(&mut self, app: &mut crate::domain::app::CrankyApp) -> Result<(), DisplayServerError>;
}

pub trait WindowManagerPort: Send + Sync {
    fn get_state(&self) -> Result<(Vec<crate::domain::workspace::Workspace>, Vec<crate::domain::workspace::Monitor>), WindowManagerError>;
}
