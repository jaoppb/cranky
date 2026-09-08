use crate::features::systray::domain::command::{CreateSystrayItemCommand, SystrayItemParams};
use crate::features::systray::domain::enums::{SystrayCategory, SystrayStatus};
use crate::features::systray::domain::icon::{IconName, SystrayIcon};
use crate::features::systray::domain::identifiers::{
    Destination, ItemId, ItemIsMenu, ObjectPath, SystrayId, Title, WindowId,
};
use crate::features::systray::domain::item::SystrayItem;
use crate::features::systray::domain::state::SystrayState;
use crate::features::systray::domain::tooltip::{
    SystrayTooltip, SystrayTooltipDescription, SystrayTooltipTitle,
};

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
        SystrayItemParams::new(
            SystrayId::new("1"),
            Destination::new("dest"),
            ObjectPath::new("/"),
            Title::new("t"),
        )
        .with_status(SystrayStatus::Active)
        .with_icon(icon.clone())
        .with_menu_path(Some(ObjectPath::new("/menu")))
        .with_category(SystrayCategory::ApplicationStatus)
        .with_item_is_menu(ItemIsMenu::new(true)),
    )
    .with_item_id(Some(ItemId::new("telegram")))
    .with_window_id(Some(WindowId::new(1234)))
    .with_attention_icon(att_icon.clone())
    .with_overlay_icon(ovr_icon.clone())
    .with_tooltip(Some(tip.clone()));

    assert_eq!(cmd.id(), &SystrayId::new("1"));
    assert_eq!(cmd.destination(), &Destination::new("dest"));
    assert_eq!(cmd.path(), &ObjectPath::new("/"));
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
        SystrayItemParams::new(
            SystrayId::new("test_id"),
            Destination::new("test_dest"),
            ObjectPath::new("/test"),
            Title::new("test_title"),
        )
        .with_status(SystrayStatus::Active)
        .with_icon(Some(SystrayIcon::new(Some(IconName::new("test_icon")), None).unwrap()))
        .with_category(SystrayCategory::Communications)
        .with_item_is_menu(ItemIsMenu::new(true)),
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
