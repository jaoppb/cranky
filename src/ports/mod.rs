pub mod canvas;

use crate::domain::errors::PortError;

pub trait DisplayServerPort: Send + Sync {
    fn create_bar(&self, output_id: u32, name: &str) -> Result<(), PortError>;
    fn destroy_bar(&self, output_id: u32) -> Result<(), PortError>;
}

pub trait WindowManagerPort: Send + Sync {
    fn get_state(&self) -> Result<(Vec<crate::core::hyprland::Workspace>, Vec<crate::core::hyprland::Monitor>), PortError>;
}
