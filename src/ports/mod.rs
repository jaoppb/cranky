pub mod canvas;
pub mod font;
pub mod dbus;
pub mod sni;

pub use dbus::DBusPort;

use crate::domain::errors::PortError;
use async_trait::async_trait;

#[async_trait]
pub trait DisplayServerPort: Send + Sync {
    fn create_bar(&self, output_id: u32, name: &str) -> Result<(), PortError>;
    fn destroy_bar(&self, output_id: u32) -> Result<(), PortError>;

    // Suspend execution until the display server has events to process
    async fn wait_for_events(&mut self) -> Result<(), PortError>;

    // Process pending events from the display server
    fn dispatch_pending(&mut self) -> Result<(), PortError>;

    // Flush the local buffer to the display server
    fn flush(&mut self) -> Result<(), PortError>;

    // Execute a full render pass for all managed outputs
    fn render_all(&mut self, app: &mut crate::domain::app::CrankyApp) -> Result<(), PortError>;
}

pub trait WindowManagerPort: Send + Sync {
    fn get_state(&self) -> Result<(Vec<crate::domain::workspace::Workspace>, Vec<crate::domain::workspace::Monitor>), PortError>;
}
