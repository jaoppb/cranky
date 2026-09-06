use crate::features::systray::adapters::icon_resolver::{
    resolve_icon, InMemorySystrayIconCache, ICON_CACHE,
};
use crate::features::systray::domain::{
    IconCacheKey, IconImage, IconName, IconThemePath,
};
use crate::features::systray::ports::SystrayIconCachePort;

#[test]
fn test_in_memory_systray_icon_cache() {
    let cache = InMemorySystrayIconCache::new();
    let key = IconCacheKey::new(
        IconName::new("test-app"),
        Some(IconThemePath::new("/custom/path")),
    );

    assert_eq!(cache.get(&key), None);

    let icon_img = IconImage::new(
        vec![1, 2, 3, 4],
        crate::shared::primitives::geometry::Size::new(1, 1),
    );
    cache.insert(key.clone(), Some(icon_img.clone()));

    assert_eq!(cache.get(&key), Some(Some(icon_img)));
}

#[tokio::test]
async fn test_resolve_icon_dynamic_pixmap_updates_not_stale() {
    let pixmap1 = vec![(1, 1, vec![255, 10, 20, 30])];
    let pixmap2 = vec![(1, 1, vec![255, 99, 88, 77])];

    let icon_name = Some("custom-applet-nonexistent-12345".to_string());

    let icon1 = resolve_icon(icon_name.clone(), None, Some(pixmap1)).await;
    assert!(icon1.is_some());
    let img1 = icon1.unwrap();
    assert_eq!(img1.data(), &[10, 20, 30, 255]);

    let icon2 = resolve_icon(icon_name.clone(), None, Some(pixmap2)).await;
    assert!(icon2.is_some());
    let img2 = icon2.unwrap();
    assert_eq!(img2.data(), &[99, 88, 77, 255]);
}

#[tokio::test]
async fn test_resolve_icon_negative_cache_allows_pixmap_fallback() {
    let icon_name = Some("nonexistent-theme-icon-67890".to_string());
    let icon_none = resolve_icon(icon_name.clone(), None, None).await;
    assert!(icon_none.is_none());

    let key = IconCacheKey::new(IconName::new("nonexistent-theme-icon-67890"), None);
    assert_eq!(ICON_CACHE.get(&key), Some(None));

    let pixmap = vec![(1, 1, vec![255, 5, 15, 25])];
    let icon_with_pixmap = resolve_icon(icon_name, None, Some(pixmap)).await;
    assert!(icon_with_pixmap.is_some());
    assert_eq!(icon_with_pixmap.unwrap().data(), &[5, 15, 25, 255]);
}

#[tokio::test]
async fn test_resolve_icon_empty_string_name_handled() {
    let pixmap = vec![(1, 1, vec![255, 1, 2, 3])];
    let icon = resolve_icon(Some("   ".to_string()), Some(String::new()), Some(pixmap)).await;
    assert!(icon.is_some());
    assert_eq!(icon.unwrap().data(), &[1, 2, 3, 255]);

    let empty_key = IconCacheKey::new(IconName::new(""), None);
    assert_eq!(ICON_CACHE.get(&empty_key), None);
}
