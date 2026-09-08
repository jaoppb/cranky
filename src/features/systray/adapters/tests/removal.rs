use crate::features::systray::adapters::watcher::Watcher;
use crate::features::systray::domain::{
    CreateSystrayItemCommand, Destination, ItemIsMenu, ObjectPath, SystrayCategory, SystrayId,
    SystrayItem, SystrayItemParams, SystrayStatus, Title,
};
use crate::shared::config::domain::Config;
use crate::shared::events::signals::SignalHub;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test]
async fn test_remove_by_destination_removes_matching_items() {
    let hub = Arc::new(SignalHub::new(Config::default()));
    let mut map = BTreeMap::new();

    let item1 = SystrayItem::new(CreateSystrayItemCommand::new(
        SystrayItemParams::new(
            SystrayId::new("app1"),
            Destination::new(":1.42"),
            ObjectPath::new("/StatusNotifierItem"),
            Title::new("App 1"),
        )
        .with_status(SystrayStatus::Active)
        .with_category(SystrayCategory::ApplicationStatus)
        .with_item_is_menu(ItemIsMenu::new(false)),
    ));
    let item2 = SystrayItem::new(CreateSystrayItemCommand::new(
        SystrayItemParams::new(
            SystrayId::new("app2"),
            Destination::new(":1.42"),
            ObjectPath::new("/StatusNotifierItem2"),
            Title::new("App 1 secondary"),
        )
        .with_status(SystrayStatus::Active)
        .with_category(SystrayCategory::ApplicationStatus)
        .with_item_is_menu(ItemIsMenu::new(false)),
    ));
    let item3 = SystrayItem::new(CreateSystrayItemCommand::new(
        SystrayItemParams::new(
            SystrayId::new("app3"),
            Destination::new(":1.43"),
            ObjectPath::new("/StatusNotifierItem"),
            Title::new("App 2"),
        )
        .with_status(SystrayStatus::Active)
        .with_category(SystrayCategory::ApplicationStatus)
        .with_item_is_menu(ItemIsMenu::new(false)),
    ));

    map.insert(SystrayId::new("app1"), item1);
    map.insert(SystrayId::new("app2"), item2);
    map.insert(SystrayId::new("app3"), item3);

    let items = Arc::new(RwLock::new(map));

    let removed = Watcher::remove_by_destination(&items, &hub, ":1.42").await;
    assert!(removed);

    let lock = items.read().await;
    assert_eq!(lock.len(), 1);
    assert!(lock.contains_key(&SystrayId::new("app3")));
    assert!(!lock.contains_key(&SystrayId::new("app1")));
    assert!(!lock.contains_key(&SystrayId::new("app2")));
    drop(lock);

    let state = hub.systray_rx().borrow().clone();
    assert_eq!(state.items().len(), 1);
    assert!(state.items().contains_key(&SystrayId::new("app3")));
}

#[tokio::test]
async fn test_remove_by_destination_no_match_returns_false() {
    let hub = Arc::new(SignalHub::new(Config::default()));
    let mut map = BTreeMap::new();

    let item = SystrayItem::new(CreateSystrayItemCommand::new(
        SystrayItemParams::new(
            SystrayId::new("app1"),
            Destination::new(":1.42"),
            ObjectPath::new("/StatusNotifierItem"),
            Title::new("App 1"),
        )
        .with_status(SystrayStatus::Active)
        .with_category(SystrayCategory::ApplicationStatus)
        .with_item_is_menu(ItemIsMenu::new(false)),
    ));
    map.insert(SystrayId::new("app1"), item);

    let items = Arc::new(RwLock::new(map));

    let removed = Watcher::remove_by_destination(&items, &hub, ":1.99").await;
    assert!(!removed);

    assert_eq!(items.read().await.len(), 1);
}
