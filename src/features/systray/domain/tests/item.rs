use crate::features::systray::domain::command::{CreateSystrayItemCommand, SystrayItemParams};
use crate::features::systray::domain::enums::{SystrayCategory, SystrayStatus};
use crate::features::systray::domain::icon::{IconName, SystrayIcon};
use crate::features::systray::domain::identifiers::{
    Destination, ItemId, ItemIsMenu, ObjectPath, SystrayAddress, SystrayId, Title, WindowId,
};
use crate::features::systray::domain::item::SystrayItem;
use crate::features::systray::domain::state::SystrayState;
use crate::features::systray::domain::tooltip::{
    SystrayTooltip, SystrayTooltipDescription, SystrayTooltipTitle,
};

/// The icons and tooltip the item of `test_systray_item` carries.
struct Decorations {
    icon: Option<SystrayIcon>,
    att_icon: Option<SystrayIcon>,
    ovr_icon: Option<SystrayIcon>,
    tip: SystrayTooltip,
}

impl Decorations {
    fn new() -> Self {
        Self {
            icon: SystrayIcon::new(Some(IconName::new("app-icon")), None),
            att_icon: SystrayIcon::new(Some(IconName::new("att-icon")), None),
            ovr_icon: SystrayIcon::new(Some(IconName::new("ovr-icon")), None),
            tip: SystrayTooltip::new(
                None,
                SystrayTooltipTitle::new("Tip"),
                SystrayTooltipDescription::new("Desc"),
            ),
        }
    }

    /// A command that sets every property of an item, with these icons and tooltip.
    fn command(&self) -> CreateSystrayItemCommand {
        CreateSystrayItemCommand::new(
            SystrayItemParams::new(
                SystrayId::new("1"),
                SystrayAddress::new(Destination::new("dest"), ObjectPath::new("/")),
                Title::new("t"),
            )
            .with_status(SystrayStatus::Active)
            .with_icon(self.icon.clone())
            .with_menu_path(Some(ObjectPath::new("/menu")))
            .with_category(SystrayCategory::ApplicationStatus)
            .with_item_is_menu(ItemIsMenu::MenuOnly),
        )
        .with_item_id(Some(ItemId::new("telegram")))
        .with_window_id(Some(WindowId::new(1234)))
        .with_attention_icon(self.att_icon.clone())
        .with_overlay_icon(self.ovr_icon.clone())
        .with_tooltip(Some(self.tip.clone()))
    }

    /// The icons and tooltip as an item's getters return them.
    fn as_refs(
        &self,
    ) -> (
        Option<&SystrayIcon>,
        Option<&SystrayIcon>,
        Option<&SystrayIcon>,
        Option<&SystrayTooltip>,
    ) {
        (
            self.icon.as_ref(),
            self.att_icon.as_ref(),
            self.ovr_icon.as_ref(),
            Some(&self.tip),
        )
    }
}

/// The id, destination and object path of the item of `test_systray_item`.
fn address() -> (SystrayId, Destination, ObjectPath) {
    (
        SystrayId::new("1"),
        Destination::new("dest"),
        ObjectPath::new("/"),
    )
}

#[test]
fn test_systray_item() {
    let decorations = Decorations::new();
    let cmd = decorations.command();
    let (id, destination, path) = address();
    assert_eq!(
        (cmd.id(), cmd.destination(), cmd.path()),
        (&id, &destination, &path)
    );
    assert_eq!(
        (cmd.status(), cmd.menu_path(), cmd.item_id(), cmd.category()),
        (
            &SystrayStatus::Active,
            Some(&ObjectPath::new("/menu")),
            Some(&ItemId::new("telegram")),
            &SystrayCategory::ApplicationStatus
        )
    );
    let window = (Some(WindowId::new(1234)), ItemIsMenu::MenuOnly);
    assert_eq!((cmd.window_id(), cmd.item_is_menu()), window);
    let shown = (
        cmd.icon(),
        cmd.attention_icon(),
        cmd.overlay_icon(),
        cmd.tooltip(),
    );
    assert_eq!(shown, decorations.as_refs());
}

#[test]
fn test_systray_item_takes_the_command_fields() {
    let decorations = Decorations::new();
    let item = SystrayItem::new(decorations.command());
    let (id, destination, path) = address();
    assert_eq!(
        (item.id(), item.destination(), item.path()),
        (&id, &destination, &path)
    );
    let ids = (Some(&ItemId::new("telegram")), Some(WindowId::new(1234)));
    assert_eq!((item.item_id(), item.window_id()), ids);
    let kind = (&SystrayCategory::ApplicationStatus, ItemIsMenu::MenuOnly);
    assert_eq!((item.category(), item.item_is_menu()), kind);
    let shown = (
        item.icon(),
        item.attention_icon(),
        item.overlay_icon(),
        item.tooltip(),
    );
    assert_eq!(shown, decorations.as_refs());
}

#[test]
fn test_systray_item_with_updates() {
    let item = SystrayItem::new(Decorations::new().command());
    let updated = item
        .with_title("t2".into())
        .with_status(SystrayStatus::Passive)
        .with_menu_path(Some(ObjectPath::new("/menu2")))
        .with_attention_icon(None)
        .with_overlay_icon(None)
        .with_tooltip(None);
    assert_eq!(
        (updated.status(), updated.menu_path()),
        (&SystrayStatus::Passive, Some(&ObjectPath::new("/menu2")))
    );
    let cleared = (
        updated.attention_icon(),
        updated.overlay_icon(),
        updated.tooltip(),
    );
    assert_eq!(cleared, (None, None, None));
}

/// The item `test_systray_state_serde` round-trips.
fn serde_item() -> SystrayItem {
    let cmd = CreateSystrayItemCommand::new(
        SystrayItemParams::new(
            SystrayId::new("test_id"),
            SystrayAddress::new(Destination::new("test_dest"), ObjectPath::new("/test")),
            Title::new("test_title"),
        )
        .with_status(SystrayStatus::Active)
        .with_icon(Some(
            SystrayIcon::new(Some(IconName::new("test_icon")), None).unwrap(),
        ))
        .with_category(SystrayCategory::Communications)
        .with_item_is_menu(ItemIsMenu::MenuOnly),
    )
    .with_item_id(Some(ItemId::new("test_item_id")))
    .with_window_id(Some(WindowId::new(101)))
    .with_tooltip(Some(SystrayTooltip::new(
        None,
        SystrayTooltipTitle::new("Tooltip Title"),
        SystrayTooltipDescription::new("Tooltip Desc"),
    )));
    SystrayItem::new(cmd)
}

#[test]
fn test_systray_state_serde() {
    let mut items = std::collections::BTreeMap::new();
    items.insert(SystrayId::new("test_id"), serde_item());
    let state = SystrayState::new(items);

    let json = serde_json::to_string(&state).unwrap();
    let decoded: SystrayState = serde_json::from_str(&json).unwrap();
    assert_eq!(state.items().len(), decoded.items().len());
    assert_eq!(
        decoded.items().get(&SystrayId::new("test_id")),
        state.items().get(&SystrayId::new("test_id"))
    );
}
