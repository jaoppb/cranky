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

pub struct SystrayItemParams {
    id: SystrayId,
    destination: Destination,
    path: ObjectPath,
    title: Title,
    status: SystrayStatus,
    icon: Option<SystrayIcon>,
    menu_path: Option<ObjectPath>,
    category: SystrayCategory,
    item_is_menu: ItemIsMenu,
}

impl SystrayItemParams {
    #[must_use]
    pub const fn new(id: SystrayId, destination: Destination, path: ObjectPath, title: Title) -> Self {
        Self {
            id,
            destination,
            path,
            title,
            status: SystrayStatus::Passive,
            icon: None,
            menu_path: None,
            category: SystrayCategory::ApplicationStatus,
            item_is_menu: ItemIsMenu::new(false),
        }
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
    pub fn with_category(mut self, category: SystrayCategory) -> Self {
        self.category = category;
        self
    }

    #[must_use]
    pub const fn with_item_is_menu(mut self, item_is_menu: ItemIsMenu) -> Self {
        self.item_is_menu = item_is_menu;
        self
    }

    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        SystrayId,
        Destination,
        ObjectPath,
        Title,
        SystrayStatus,
        Option<SystrayIcon>,
        Option<ObjectPath>,
        SystrayCategory,
        ItemIsMenu,
    ) {
        (
            self.id,
            self.destination,
            self.path,
            self.title,
            self.status,
            self.icon,
            self.menu_path,
            self.category,
            self.item_is_menu,
        )
    }
}

impl CreateSystrayItemCommand {
    #[must_use]
    pub fn new(params: SystrayItemParams) -> Self {
        let (id, destination, path, title, status, icon, menu_path, category, item_is_menu) =
            params.into_parts();
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
