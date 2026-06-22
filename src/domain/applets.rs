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
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Destination(String);

impl Destination {
    pub fn new(dest: impl Into<String>) -> Self {
        Self(dest.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObjectPath(String);

impl ObjectPath {
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Title(String);

impl Title {
    pub fn new(title: impl Into<String>) -> Self {
        Self(title.into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IconName(String);

impl IconName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
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


pub struct CreateAppletCommand {
    pub id: AppletId,
    pub destination: Destination,
    pub path: ObjectPath,
    pub title: Title,
    pub status: AppletStatus,
    pub icon_name: Option<IconName>,
    pub icon_image: Option<IconImage>,
    pub menu_path: Option<ObjectPath>,
}

impl AppletItem {
    pub fn new(cmd: CreateAppletCommand) -> Self {
        Self {
            id: cmd.id,
            destination: cmd.destination,
            path: cmd.path,
            title: cmd.title,
            status: cmd.status,
            icon_name: cmd.icon_name,
            icon_image: cmd.icon_image,
            menu_path: cmd.menu_path,
        }
    }

    pub fn destination(&self) -> &Destination {
        &self.destination
    }
    pub fn path(&self) -> &ObjectPath {
        &self.path
    }
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
