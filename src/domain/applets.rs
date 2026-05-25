use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AppletStatus {
    Active,
    Passive,
    NeedsAttention,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppletItem {
    pub id: String,
    pub destination: String,
    pub path: String,
    pub title: String,
    pub status: AppletStatus,
    /// Identifier for the icon theme, if provided by the applet
    pub icon_name: Option<String>,
    /// Rasterized RGBA data from either `icon_name` resolution or raw `icon_pixmap`
    pub icon_data: Option<Vec<u8>>,
    /// Width of the `icon_data` image
    pub icon_width: u32,
    /// Height of the `icon_data` image
    pub icon_height: u32,
    pub menu_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct AppletsState {
    pub items: Vec<AppletItem>,
}
