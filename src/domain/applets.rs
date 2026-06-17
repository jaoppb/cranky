use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AppletStatus {
    Active,
    Passive,
    NeedsAttention,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppletId(String);

impl AppletId {
    pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Destination(String);

impl Destination {
    pub fn new(dest: impl Into<String>) -> Self { Self(dest.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObjectPath(String);

impl ObjectPath {
    pub fn new(path: impl Into<String>) -> Self { Self(path.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Title(String);

impl Title {
    pub fn new(title: impl Into<String>) -> Self { Self(title.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IconName(String);

impl IconName {
    pub fn new(name: impl Into<String>) -> Self { Self(name.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

use crate::domain::shared::geometry::Size;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IconImage {
    data: Vec<u8>,
    size: Size,
}

impl IconImage {
    pub fn new(data: Vec<u8>, size: Size) -> Self {
        Self { data, size }
    }

    pub fn data(&self) -> &[u8] { &self.data }
    pub fn size(&self) -> &Size { &self.size }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppletItem {
    id: AppletId,
    destination: Destination,
    path: ObjectPath,
    title: Title,
    status: AppletStatus,
    icon_name: Option<IconName>,
    icon_image: Option<IconImage>,
    menu_path: Option<ObjectPath>,
}

impl AppletItem {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: AppletId,
        destination: Destination,
        path: ObjectPath,
        title: Title,
        status: AppletStatus,
        icon_name: Option<IconName>,
        icon_image: Option<IconImage>,
        menu_path: Option<ObjectPath>,
    ) -> Self {
        Self {
            id,
            destination,
            path,
            title,
            status,
            icon_name,
            icon_image,
            menu_path,
        }
    }

    pub fn id(&self) -> &AppletId { &self.id }
    pub fn destination(&self) -> &Destination { &self.destination }
    pub fn path(&self) -> &ObjectPath { &self.path }
    pub fn title(&self) -> &Title { &self.title }
    pub fn status(&self) -> &AppletStatus { &self.status }
    pub fn icon_name(&self) -> Option<&IconName> { self.icon_name.as_ref() }
    pub fn icon_image(&self) -> Option<&IconImage> { self.icon_image.as_ref() }
    pub fn menu_path(&self) -> Option<&ObjectPath> { self.menu_path.as_ref() }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct AppletsState {
    items: Vec<AppletItem>,
}

impl AppletsState {
    pub fn new(items: Vec<AppletItem>) -> Self {
        Self { items }
    }

    pub fn items(&self) -> &[AppletItem] {
        &self.items
    }
}
