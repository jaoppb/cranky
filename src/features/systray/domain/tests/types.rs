use crate::features::systray::domain::enums::{SystrayActionName, SystrayCategory};
use crate::features::systray::domain::icon::{IconImage, IconName, SystrayIcon};
use crate::features::systray::domain::identifiers::{
    Destination, ItemId, ItemIsMenu, ObjectPath, SystrayId, Title, WindowId,
};
use crate::features::systray::domain::tooltip::{
    SystrayTooltip, SystrayTooltipDescription, SystrayTooltipTitle,
};
use crate::shared::primitives::geometry::Size;

#[test]
fn test_systray_types() {
    assert_eq!(SystrayId::new("id").as_str(), "id");
    assert_eq!(Destination::new("dest").as_str(), "dest");
    assert_eq!(ObjectPath::new("/path").as_str(), "/path");
    assert_eq!(Title::new("title"), Title::new("title"));
    assert_eq!(IconName::new("icon").as_str(), "icon");
    let size = Size::new(10, 10);
    let img = IconImage::new(vec![0], size);
    assert_eq!(img.size(), &size);
    assert_eq!(img.data(), &[0]);
}

#[test]
fn test_icon_image_debug_omission() {
    let size = Size::new(16, 16);
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
