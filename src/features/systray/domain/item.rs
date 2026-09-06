use super::command::CreateSystrayItemCommand;
use super::enums::{SystrayCategory, SystrayStatus};
use super::icon::SystrayIcon;
use super::identifiers::{Destination, ItemId, ItemIsMenu, ObjectPath, SystrayId, Title, WindowId};
use super::tooltip::SystrayTooltip;
use serde::{Deserialize, Serialize};

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
