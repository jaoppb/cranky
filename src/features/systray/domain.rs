use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SystrayStatus {
    Active,
    Passive,
    NeedsAttention,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct SystrayId(String);

impl SystrayId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for SystrayId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for SystrayId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct Destination(String);

impl Destination {
    pub fn new(dest: impl Into<String>) -> Self {
        Self(dest.into())
    }
    #[must_use]
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
    #[must_use]
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
    #[cfg(test)]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::BinaryData;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IconImage {
    data: BinaryData,
    size: Size,
}

impl IconImage {
    pub fn new(data: impl Into<BinaryData>, size: Size) -> Self {
        Self {
            data: data.into(),
            size,
        }
    }
    #[cfg(test)]
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }
    #[cfg(test)]
    #[must_use]
    pub const fn size(&self) -> &Size {
        &self.size
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct ItemId(String);

impl ItemId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    #[cfg(test)]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct WindowId(u32);

impl WindowId {
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
    #[cfg(test)]
    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(transparent)]
pub struct ItemIsMenu(bool);

impl ItemIsMenu {
    #[must_use]
    pub const fn new(val: bool) -> Self {
        Self(val)
    }
    #[must_use]
    pub const fn value(&self) -> bool {
        self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct SystrayTooltipTitle(String);

impl SystrayTooltipTitle {
    pub fn new(title: impl Into<String>) -> Self {
        Self(title.into())
    }
    #[cfg(test)]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct SystrayTooltipDescription(String);

impl SystrayTooltipDescription {
    pub fn new(desc: impl Into<String>) -> Self {
        Self(desc.into())
    }
    #[cfg(test)]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(try_from = "String", into = "String")]
pub enum SystrayActionName {
    Primary,
    ContextMenu,
    Activate,
    SecondaryActivate,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
    Other(String),
}

impl SystrayActionName {
    #[must_use]
    pub fn parse_str(s: &str) -> Self {
        match s {
            "Primary" => Self::Primary,
            "ContextMenu" => Self::ContextMenu,
            "Activate" => Self::Activate,
            "SecondaryActivate" => Self::SecondaryActivate,
            "ScrollUp" => Self::ScrollUp,
            "ScrollDown" => Self::ScrollDown,
            "ScrollLeft" => Self::ScrollLeft,
            "ScrollRight" => Self::ScrollRight,
            other => Self::Other(other.to_string()),
        }
    }

    #[must_use]
    pub const fn as_str(&self) -> &str {
        match self {
            Self::Primary => "Primary",
            Self::ContextMenu => "ContextMenu",
            Self::Activate => "Activate",
            Self::SecondaryActivate => "SecondaryActivate",
            Self::ScrollUp => "ScrollUp",
            Self::ScrollDown => "ScrollDown",
            Self::ScrollLeft => "ScrollLeft",
            Self::ScrollRight => "ScrollRight",
            Self::Other(other) => other.as_str(),
        }
    }
}

impl From<&str> for SystrayActionName {
    fn from(s: &str) -> Self {
        Self::parse_str(s)
    }
}

impl From<String> for SystrayActionName {
    fn from(s: String) -> Self {
        Self::parse_str(&s)
    }
}

impl From<SystrayActionName> for String {
    fn from(action: SystrayActionName) -> Self {
        action.as_str().to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(try_from = "String", into = "String")]
pub enum SystrayCategory {
    ApplicationStatus,
    Communications,
    SystemServices,
    Hardware,
    Other(String),
}

impl SystrayCategory {
    #[must_use]
    pub fn parse_str(s: &str) -> Self {
        match s {
            "ApplicationStatus" => Self::ApplicationStatus,
            "Communications" => Self::Communications,
            "SystemServices" => Self::SystemServices,
            "Hardware" => Self::Hardware,
            other => Self::Other(other.to_string()),
        }
    }

    #[must_use]
    pub const fn as_str(&self) -> &str {
        match self {
            Self::ApplicationStatus => "ApplicationStatus",
            Self::Communications => "Communications",
            Self::SystemServices => "SystemServices",
            Self::Hardware => "Hardware",
            Self::Other(other) => other.as_str(),
        }
    }
}

impl From<&str> for SystrayCategory {
    fn from(s: &str) -> Self {
        Self::parse_str(s)
    }
}

impl From<String> for SystrayCategory {
    fn from(s: String) -> Self {
        Self::parse_str(&s)
    }
}

impl From<SystrayCategory> for String {
    fn from(cat: SystrayCategory) -> Self {
        cat.as_str().to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum SystrayIcon {
    Both { name: IconName, image: IconImage },
    NameOnly { name: IconName },
    ImageOnly { image: IconImage },
}

impl SystrayIcon {
    #[must_use]
    pub fn new(name: Option<IconName>, image: Option<IconImage>) -> Option<Self> {
        match (name, image) {
            (Some(name), Some(image)) => Some(Self::Both { name, image }),
            (Some(name), None) => Some(Self::NameOnly { name }),
            (None, Some(image)) => Some(Self::ImageOnly { image }),
            (None, None) => None,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub const fn name(&self) -> Option<&IconName> {
        match self {
            Self::Both { name, .. } | Self::NameOnly { name } => Some(name),
            Self::ImageOnly { .. } => None,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub const fn image(&self) -> Option<&IconImage> {
        match self {
            Self::Both { image, .. } | Self::ImageOnly { image } => Some(image),
            Self::NameOnly { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystrayTooltip {
    icon: Option<SystrayIcon>,
    title: SystrayTooltipTitle,
    description: SystrayTooltipDescription,
}

impl SystrayTooltip {
    #[must_use]
    pub const fn new(
        icon: Option<SystrayIcon>,
        title: SystrayTooltipTitle,
        description: SystrayTooltipDescription,
    ) -> Self {
        Self {
            icon,
            title,
            description,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub const fn icon(&self) -> Option<&SystrayIcon> {
        self.icon.as_ref()
    }

    #[cfg(test)]
    #[must_use]
    pub const fn title(&self) -> &SystrayTooltipTitle {
        &self.title
    }

    #[cfg(test)]
    #[must_use]
    pub const fn description(&self) -> &SystrayTooltipDescription {
        &self.description
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystrayItem {
    id: SystrayId,
    destination: Destination,
    path: ObjectPath,
    title: Title,
    status: SystrayStatus,
    icon: Option<SystrayIcon>,
    menu_path: Option<ObjectPath>,
    item_id: Option<ItemId>,
    category: SystrayCategory,
    window_id: Option<WindowId>,
    item_is_menu: ItemIsMenu,
    attention_icon: Option<SystrayIcon>,
    overlay_icon: Option<SystrayIcon>,
    tooltip: Option<SystrayTooltip>,
}

pub struct CreateSystrayItemCommand {
    id: SystrayId,
    destination: Destination,
    path: ObjectPath,
    title: Title,
    status: SystrayStatus,
    icon: Option<SystrayIcon>,
    menu_path: Option<ObjectPath>,
    item_id: Option<ItemId>,
    category: SystrayCategory,
    window_id: Option<WindowId>,
    item_is_menu: ItemIsMenu,
    attention_icon: Option<SystrayIcon>,
    overlay_icon: Option<SystrayIcon>,
    tooltip: Option<SystrayTooltip>,
}

impl CreateSystrayItemCommand {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub const fn new(
        id: SystrayId,
        destination: Destination,
        path: ObjectPath,
        title: Title,
        status: SystrayStatus,
        icon: Option<SystrayIcon>,
        menu_path: Option<ObjectPath>,
        category: SystrayCategory,
        item_is_menu: ItemIsMenu,
    ) -> Self {
        Self {
            id,
            destination,
            path,
            title,
            status,
            icon,
            menu_path,
            item_id: None,
            category,
            window_id: None,
            item_is_menu,
            attention_icon: None,
            overlay_icon: None,
            tooltip: None,
        }
    }

    #[must_use]
    pub fn with_item_id(mut self, item_id: Option<ItemId>) -> Self {
        self.item_id = item_id;
        self
    }

    #[must_use]
    pub const fn with_window_id(mut self, window_id: Option<WindowId>) -> Self {
        self.window_id = window_id;
        self
    }

    #[must_use]
    pub fn with_attention_icon(mut self, attention_icon: Option<SystrayIcon>) -> Self {
        self.attention_icon = attention_icon;
        self
    }

    #[must_use]
    pub fn with_overlay_icon(mut self, overlay_icon: Option<SystrayIcon>) -> Self {
        self.overlay_icon = overlay_icon;
        self
    }

    #[must_use]
    pub fn with_tooltip(mut self, tooltip: Option<SystrayTooltip>) -> Self {
        self.tooltip = tooltip;
        self
    }

    #[cfg(test)]
    #[must_use]
    pub const fn id(&self) -> &SystrayId {
        &self.id
    }
    #[cfg(test)]
    #[must_use]
    pub const fn destination(&self) -> &Destination {
        &self.destination
    }
    #[cfg(test)]
    #[must_use]
    pub const fn path(&self) -> &ObjectPath {
        &self.path
    }
    #[cfg(test)]
    #[must_use]
    pub const fn title(&self) -> &Title {
        &self.title
    }
    #[cfg(test)]
    #[must_use]
    pub const fn status(&self) -> &SystrayStatus {
        &self.status
    }
    #[cfg(test)]
    #[must_use]
    pub const fn icon(&self) -> Option<&SystrayIcon> {
        self.icon.as_ref()
    }
    #[cfg(test)]
    #[must_use]
    pub const fn menu_path(&self) -> Option<&ObjectPath> {
        self.menu_path.as_ref()
    }
    #[cfg(test)]
    #[must_use]
    pub const fn item_id(&self) -> Option<&ItemId> {
        self.item_id.as_ref()
    }
    #[cfg(test)]
    #[must_use]
    pub const fn category(&self) -> &SystrayCategory {
        &self.category
    }
    #[cfg(test)]
    #[must_use]
    pub const fn window_id(&self) -> Option<WindowId> {
        self.window_id
    }
    #[cfg(test)]
    #[must_use]
    pub const fn item_is_menu(&self) -> ItemIsMenu {
        self.item_is_menu
    }
    #[cfg(test)]
    #[must_use]
    pub const fn attention_icon(&self) -> Option<&SystrayIcon> {
        self.attention_icon.as_ref()
    }
    #[cfg(test)]
    #[must_use]
    pub const fn overlay_icon(&self) -> Option<&SystrayIcon> {
        self.overlay_icon.as_ref()
    }
    #[cfg(test)]
    #[must_use]
    pub const fn tooltip(&self) -> Option<&SystrayTooltip> {
        self.tooltip.as_ref()
    }
}

impl SystrayItem {
    #[must_use]
    pub fn new(cmd: CreateSystrayItemCommand) -> Self {
        Self {
            id: cmd.id,
            destination: cmd.destination,
            path: cmd.path,
            title: cmd.title,
            status: cmd.status,
            icon: cmd.icon,
            menu_path: cmd.menu_path,
            item_id: cmd.item_id,
            category: cmd.category,
            window_id: cmd.window_id,
            item_is_menu: cmd.item_is_menu,
            attention_icon: cmd.attention_icon,
            overlay_icon: cmd.overlay_icon,
            tooltip: cmd.tooltip,
        }
    }

    #[must_use]
    pub fn with_title(mut self, title: String) -> Self {
        self.title = Title::new(title);
        self
    }

    #[must_use]
    pub const fn with_status(mut self, status: SystrayStatus) -> Self {
        self.status = status;
        self
    }

    #[must_use]
    pub fn with_icon(mut self, icon: Option<SystrayIcon>) -> Self {
        self.icon = icon;
        self
    }

    #[must_use]
    pub fn with_menu_path(mut self, menu_path: Option<ObjectPath>) -> Self {
        self.menu_path = menu_path;
        self
    }

    #[must_use]
    pub const fn with_item_is_menu(mut self, item_is_menu: ItemIsMenu) -> Self {
        self.item_is_menu = item_is_menu;
        self
    }

    #[must_use]
    pub fn with_attention_icon(mut self, attention_icon: Option<SystrayIcon>) -> Self {
        self.attention_icon = attention_icon;
        self
    }

    #[must_use]
    pub fn with_overlay_icon(mut self, overlay_icon: Option<SystrayIcon>) -> Self {
        self.overlay_icon = overlay_icon;
        self
    }

    #[must_use]
    pub fn with_tooltip(mut self, tooltip: Option<SystrayTooltip>) -> Self {
        self.tooltip = tooltip;
        self
    }

    #[must_use]
    pub const fn id(&self) -> &SystrayId {
        &self.id
    }

    #[must_use]
    pub const fn destination(&self) -> &Destination {
        &self.destination
    }

    #[must_use]
    pub const fn path(&self) -> &ObjectPath {
        &self.path
    }

    #[cfg(test)]
    #[must_use]
    pub const fn title(&self) -> &Title {
        &self.title
    }

    #[cfg(test)]
    #[must_use]
    pub const fn status(&self) -> &SystrayStatus {
        &self.status
    }

    #[cfg(test)]
    #[must_use]
    pub const fn icon(&self) -> Option<&SystrayIcon> {
        self.icon.as_ref()
    }

    #[cfg(test)]
    #[must_use]
    pub const fn menu_path(&self) -> Option<&ObjectPath> {
        self.menu_path.as_ref()
    }

    #[cfg(test)]
    #[must_use]
    pub const fn item_id(&self) -> Option<&ItemId> {
        self.item_id.as_ref()
    }

    #[cfg(test)]
    #[must_use]
    pub const fn category(&self) -> &SystrayCategory {
        &self.category
    }

    #[cfg(test)]
    #[must_use]
    pub const fn window_id(&self) -> Option<WindowId> {
        self.window_id
    }

    #[must_use]
    pub const fn item_is_menu(&self) -> ItemIsMenu {
        self.item_is_menu
    }

    #[cfg(test)]
    #[must_use]
    pub const fn attention_icon(&self) -> Option<&SystrayIcon> {
        self.attention_icon.as_ref()
    }

    #[cfg(test)]
    #[must_use]
    pub const fn overlay_icon(&self) -> Option<&SystrayIcon> {
        self.overlay_icon.as_ref()
    }

    #[cfg(test)]
    #[must_use]
    pub const fn tooltip(&self) -> Option<&SystrayTooltip> {
        self.tooltip.as_ref()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SystrayState {
    items: std::collections::BTreeMap<SystrayId, SystrayItem>,
}

impl serde::Serialize for SystrayState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SystrayState", 1)?;
        let items: Vec<&SystrayItem> = self.items.values().collect();
        state.serialize_field("items", &items)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for SystrayState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Helper {
            items: Vec<SystrayItem>,
        }
        let helper = Helper::deserialize(deserializer)?;
        let mut items = std::collections::BTreeMap::new();
        for item in helper.items {
            items.insert(item.id().clone(), item);
        }
        Ok(Self { items })
    }
}

impl SystrayState {
    #[must_use]
    pub const fn new(items: std::collections::BTreeMap<SystrayId, SystrayItem>) -> Self {
        Self { items }
    }

    #[must_use]
    pub const fn items(&self) -> &std::collections::BTreeMap<SystrayId, SystrayItem> {
        &self.items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_systray_types() {
        assert_eq!(SystrayId::new("id").as_str(), "id");
        assert_eq!(Destination::new("dest").as_str(), "dest");
        assert_eq!(ObjectPath::new("/path").as_str(), "/path");
        assert_eq!(Title::new("title"), Title("title".into()));
        assert_eq!(IconName::new("icon").as_str(), "icon");
        let size = crate::shared::primitives::geometry::Size::new(10, 10);
        let img = IconImage::new(vec![0], size);
        assert_eq!(img.size(), &size);
        assert_eq!(img.data(), &[0]);
    }

    #[test]
    fn test_icon_image_debug_omission() {
        let size = crate::shared::primitives::geometry::Size::new(16, 16);
        let img = IconImage::new(vec![255; 16 * 16 * 4], size);
        let debug_str = format!("{img:?}");
        assert!(debug_str.contains("<Binary Data (1024 bytes)>"));
        assert!(!debug_str.contains("255, 255"));
    }

    #[test]
    fn test_value_objects() {
        assert_eq!(ItemId::new("item1").as_str(), "item1");
        assert_eq!(WindowId::new(42).value(), 42);
        assert!(ItemIsMenu::new(true).value());
        assert_eq!(SystrayTooltipTitle::new("T").as_str(), "T");
        assert_eq!(SystrayTooltipDescription::new("D").as_str(), "D");
    }

    #[test]
    fn test_systray_action_name() {
        assert_eq!(
            SystrayActionName::parse_str("Primary"),
            SystrayActionName::Primary
        );
        assert_eq!(
            SystrayActionName::parse_str("ContextMenu"),
            SystrayActionName::ContextMenu
        );
        assert_eq!(
            SystrayActionName::parse_str("Custom"),
            SystrayActionName::Other("Custom".into())
        );
        assert_eq!(SystrayActionName::Primary.as_str(), "Primary");
        let s: String = SystrayActionName::ContextMenu.into();
        assert_eq!(s, "ContextMenu");
    }

    #[test]
    fn test_systray_category() {
        assert_eq!(
            SystrayCategory::parse_str("ApplicationStatus"),
            SystrayCategory::ApplicationStatus
        );
        assert_eq!(
            SystrayCategory::parse_str("Communications"),
            SystrayCategory::Communications
        );
        assert_eq!(
            SystrayCategory::parse_str("UnknownCat"),
            SystrayCategory::Other("UnknownCat".into())
        );
        assert_eq!(SystrayCategory::Hardware.as_str(), "Hardware");
    }

    #[test]
    fn test_systray_icon_wrapper() {
        let name = Some(IconName::new("telegram"));
        let img = Some(IconImage::new(vec![1, 2, 3, 4], Size::new(1, 1)));
        let icon_both = SystrayIcon::new(name.clone(), img.clone()).unwrap();
        assert_eq!(icon_both.name(), name.as_ref());
        assert_eq!(icon_both.image(), img.as_ref());

        let icon_name = SystrayIcon::new(name.clone(), None).unwrap();
        assert_eq!(icon_name.name(), name.as_ref());
        assert!(icon_name.image().is_none());

        let icon_img = SystrayIcon::new(None, img.clone()).unwrap();
        assert!(icon_img.name().is_none());
        assert_eq!(icon_img.image(), img.as_ref());

        assert!(SystrayIcon::new(None, None).is_none());
    }

    #[test]
    fn test_systray_tooltip() {
        let title = SystrayTooltipTitle::new("T");
        let desc = SystrayTooltipDescription::new("D");
        let tooltip = SystrayTooltip::new(None, title.clone(), desc.clone());
        assert!(tooltip.icon().is_none());
        assert_eq!(tooltip.title(), &title);
        assert_eq!(tooltip.description(), &desc);
    }

    #[test]
    fn test_systray_item() {
        let icon = SystrayIcon::new(Some(IconName::new("app-icon")), None);
        let att_icon = SystrayIcon::new(Some(IconName::new("att-icon")), None);
        let ovr_icon = SystrayIcon::new(Some(IconName::new("ovr-icon")), None);
        let tip = SystrayTooltip::new(
            None,
            SystrayTooltipTitle::new("Tip"),
            SystrayTooltipDescription::new("Desc"),
        );

        let cmd = CreateSystrayItemCommand::new(
            SystrayId::new("1"),
            Destination::new("dest"),
            ObjectPath::new("/"),
            Title::new("t"),
            SystrayStatus::Active,
            icon.clone(),
            Some(ObjectPath::new("/menu")),
            SystrayCategory::ApplicationStatus,
            ItemIsMenu::new(true),
        )
        .with_item_id(Some(ItemId::new("telegram")))
        .with_window_id(Some(WindowId::new(1234)))
        .with_attention_icon(att_icon.clone())
        .with_overlay_icon(ovr_icon.clone())
        .with_tooltip(Some(tip.clone()));

        assert_eq!(cmd.id(), &SystrayId::new("1"));
        assert_eq!(cmd.destination(), &Destination::new("dest"));
        assert_eq!(cmd.path(), &ObjectPath::new("/"));
        assert_eq!(cmd.title(), &Title::new("t"));
        assert_eq!(cmd.status(), &SystrayStatus::Active);
        assert_eq!(cmd.icon(), icon.as_ref());
        assert_eq!(cmd.menu_path(), Some(&ObjectPath::new("/menu")));
        assert_eq!(cmd.item_id(), Some(&ItemId::new("telegram")));
        assert_eq!(cmd.category(), &SystrayCategory::ApplicationStatus);
        assert_eq!(cmd.window_id(), Some(WindowId::new(1234)));
        assert_eq!(cmd.item_is_menu(), ItemIsMenu::new(true));
        assert_eq!(cmd.attention_icon(), att_icon.as_ref());
        assert_eq!(cmd.overlay_icon(), ovr_icon.as_ref());
        assert_eq!(cmd.tooltip(), Some(&tip));

        let item = SystrayItem::new(cmd);
        assert_eq!(item.id(), &SystrayId::new("1"));
        assert_eq!(item.destination(), &Destination::new("dest"));
        assert_eq!(item.path(), &ObjectPath::new("/"));
        assert_eq!(item.item_id(), Some(&ItemId::new("telegram")));
        assert_eq!(item.window_id(), Some(WindowId::new(1234)));
        assert_eq!(item.category(), &SystrayCategory::ApplicationStatus);
        assert_eq!(item.icon(), icon.as_ref());
        assert_eq!(item.item_is_menu(), ItemIsMenu::new(true));
        assert_eq!(item.attention_icon(), att_icon.as_ref());
        assert_eq!(item.overlay_icon(), ovr_icon.as_ref());
        assert_eq!(item.tooltip(), Some(&tip));

        let updated = item
            .with_title("t2".into())
            .with_status(SystrayStatus::Passive)
            .with_menu_path(Some(ObjectPath::new("/menu2")))
            .with_attention_icon(None)
            .with_overlay_icon(None)
            .with_tooltip(None);
        assert_eq!(updated.title(), &Title::new("t2"));
        assert_eq!(updated.status(), &SystrayStatus::Passive);
        assert_eq!(updated.menu_path(), Some(&ObjectPath::new("/menu2")));
        assert_eq!(updated.attention_icon(), None);
        assert_eq!(updated.overlay_icon(), None);
        assert_eq!(updated.tooltip(), None);
    }

    #[test]
    fn test_systray_state_serde() {
        let mut items = std::collections::BTreeMap::new();
        let cmd = CreateSystrayItemCommand::new(
            SystrayId::new("test_id"),
            Destination::new("test_dest"),
            ObjectPath::new("/test"),
            Title::new("test_title"),
            SystrayStatus::Active,
            Some(SystrayIcon::new(Some(IconName::new("test_icon")), None).unwrap()),
            None,
            SystrayCategory::Communications,
            ItemIsMenu::new(true),
        )
        .with_item_id(Some(ItemId::new("test_item_id")))
        .with_window_id(Some(WindowId::new(101)))
        .with_tooltip(Some(SystrayTooltip::new(
            None,
            SystrayTooltipTitle::new("Tooltip Title"),
            SystrayTooltipDescription::new("Tooltip Desc"),
        )));

        items.insert(SystrayId::new("test_id"), SystrayItem::new(cmd));
        let state = SystrayState::new(items);

        let json = serde_json::to_string(&state).unwrap();
        let decoded: SystrayState = serde_json::from_str(&json).unwrap();
        assert_eq!(state.items().len(), decoded.items().len());
        assert_eq!(
            decoded.items().get(&SystrayId::new("test_id")),
            state.items().get(&SystrayId::new("test_id"))
        );
    }
}
