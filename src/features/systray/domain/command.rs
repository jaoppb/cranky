use super::enums::{SystrayCategory, SystrayStatus};
use super::icon::SystrayIcon;
use super::identifiers::{Destination, ItemId, ItemIsMenu, ObjectPath, SystrayId, Title, WindowId};
use super::tooltip::SystrayTooltip;

pub struct CreateSystrayItemCommand {
    pub(crate) id: SystrayId,
    pub(crate) destination: Destination,
    pub(crate) path: ObjectPath,
    pub(crate) title: Title,
    pub(crate) status: SystrayStatus,
    pub(crate) icon: Option<SystrayIcon>,
    pub(crate) menu_path: Option<ObjectPath>,
    pub(crate) item_id: Option<ItemId>,
    pub(crate) category: SystrayCategory,
    pub(crate) window_id: Option<WindowId>,
    pub(crate) item_is_menu: ItemIsMenu,
    pub(crate) attention_icon: Option<SystrayIcon>,
    pub(crate) overlay_icon: Option<SystrayIcon>,
    pub(crate) tooltip: Option<SystrayTooltip>,
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
