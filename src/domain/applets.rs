use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AppletStatus {
    Active,
    Passive,
    NeedsAttention,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct AppletId(String);

impl AppletId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
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
#[serde(transparent)]
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
#[serde(transparent)]
pub struct Title(String);

impl Title {
    pub fn new(title: impl Into<String>) -> Self {
        Self(title.into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
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

    pub fn id(&self) -> &AppletId {
        &self.id
    }

    pub fn destination(&self) -> &Destination {
        &self.destination
    }
    pub fn path(&self) -> &ObjectPath {
        &self.path
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AppletsState {
    items: std::collections::BTreeMap<AppletId, AppletItem>,
}

impl serde::Serialize for AppletsState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("AppletsState", 1)?;
        let items: Vec<&AppletItem> = self.items.values().collect();
        state.serialize_field("items", &items)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for AppletsState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Helper {
            items: Vec<AppletItem>,
        }
        let helper = Helper::deserialize(deserializer)?;
        let mut items = std::collections::BTreeMap::new();
        for item in helper.items {
            items.insert(item.id().clone(), item);
        }
        Ok(Self { items })
    }
}

impl AppletsState {
    pub fn new(items: std::collections::BTreeMap<AppletId, AppletItem>) -> Self {
        Self { items }
    }

    pub fn items(&self) -> &std::collections::BTreeMap<AppletId, AppletItem> {
        &self.items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_applet_types() {
        assert_eq!(AppletId::new("id").as_str(), "id");
        assert_eq!(Destination::new("dest").as_str(), "dest");
        assert_eq!(ObjectPath::new("/path").as_str(), "/path");
        assert_eq!(Title::new("title"), Title("title".into()));
        assert_eq!(IconName::new("icon"), IconName("icon".into()));
        let size = crate::domain::shared::geometry::Size::new(10, 10);
        let img = IconImage::new(vec![0], size);
        assert_eq!(img.size, size);
    }

    #[test]
    fn test_applet_item() {
        let cmd = CreateAppletCommand {
            id: AppletId::new("1"),
            destination: Destination::new("dest"),
            path: ObjectPath::new("/"),
            title: Title::new("t"),
            status: AppletStatus::Active,
            icon_name: None,
            icon_image: None,
            menu_path: None,
        };
        let item = AppletItem::new(cmd);
        assert_eq!(item.id(), &AppletId::new("1"));
        assert_eq!(item.destination(), &Destination::new("dest"));
        assert_eq!(item.path(), &ObjectPath::new("/"));
    }

    #[test]
    fn test_applets_state_serde() {
        let mut items = std::collections::BTreeMap::new();
        let cmd = CreateAppletCommand {
            id: AppletId::new("test_id"),
            destination: Destination::new("test_dest"),
            path: ObjectPath::new("/test"),
            title: Title::new("test_title"),
            status: AppletStatus::Active,
            icon_name: None,
            icon_image: None,
            menu_path: None,
        };
        items.insert(AppletId::new("test_id"), AppletItem::new(cmd));
        let state = AppletsState::new(items);

        let json = serde_json::to_string(&state).unwrap();
        let decoded: AppletsState = serde_json::from_str(&json).unwrap();
        assert_eq!(state.items().len(), decoded.items().len());
        assert_eq!(decoded.items().get(&AppletId::new("test_id")), state.items().get(&AppletId::new("test_id")));
    }
}

