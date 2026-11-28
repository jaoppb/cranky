use crate::features::systray::adapters::tests::fixtures::item;
use crate::features::systray::adapters::tray_items::TrayItems;
use crate::features::systray::adapters::watcher::Watcher;
use crate::features::systray::domain::{
    CreateSystrayItemCommand, Destination, ItemIsMenu, ObjectPath, SystrayAddress, SystrayCategory,
    SystrayId, SystrayItem, SystrayItemParams, SystrayStatus, Title,
};
use crate::shared::config::domain::Config;
use crate::shared::events::signals::SignalHub;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[test]
fn test_remove_by_destination_removes_matching_items() {
    crate::test_utils::block_on(async {
        let hub = Arc::new(SignalHub::new(Config::default()));
        let map = BTreeMap::from([
            (SystrayId::new("app1"), item("app1", ":1.42")),
            (SystrayId::new("app2"), item("app2", ":1.42")),
            (SystrayId::new("app3"), item("app3", ":1.43")),
        ]);
        let items = Arc::new(RwLock::new(map));

        let removed =
            Watcher::remove_by_destination(&TrayItems::new(items.clone(), hub.clone()), ":1.42")
                .await;
        assert!(removed);

        let kept: Vec<SystrayId> = items.read().await.keys().cloned().collect();
        assert_eq!(kept, [SystrayId::new("app3")]);

        let state = hub.systray_rx().borrow().clone();
        let published: Vec<&SystrayId> = state.items().keys().collect();
        assert_eq!(published, [&SystrayId::new("app3")]);
    });
}

#[test]
fn test_remove_by_destination_no_match_returns_false() {
    crate::test_utils::block_on(async {
        let hub = Arc::new(SignalHub::new(Config::default()));
        let mut map = BTreeMap::new();

        let item = SystrayItem::new(CreateSystrayItemCommand::new(
            SystrayItemParams::new(
                SystrayId::new("app1"),
                SystrayAddress::new(
                    Destination::new(":1.42"),
                    ObjectPath::new("/StatusNotifierItem"),
                ),
                Title::new("App 1"),
            )
            .with_status(SystrayStatus::Active)
            .with_category(SystrayCategory::ApplicationStatus)
            .with_item_is_menu(ItemIsMenu::Activatable),
        ));
        map.insert(SystrayId::new("app1"), item);

        let items = Arc::new(RwLock::new(map));

        let removed =
            Watcher::remove_by_destination(&TrayItems::new(items.clone(), hub.clone()), ":1.99")
                .await;
        assert!(!removed);

        assert_eq!(items.read().await.len(), 1);
    });
}

#[test]
fn test_remove_by_destination_without_match_does_not_publish() {
    crate::test_utils::block_on(async {
        let hub = Arc::new(SignalHub::new(Config::default()));
        let mut map = BTreeMap::new();
        map.insert(SystrayId::new("app1"), item("app1", ":1.42"));
        let tray = TrayItems::new(Arc::new(RwLock::new(map)), hub.clone());

        assert!(!Watcher::remove_by_destination(&tray, ":1.99").await);
        // A publish would have sent the one tracked item; the hub still holds the empty state.
        assert!(hub.systray_rx().borrow().items().is_empty());
    });
}
