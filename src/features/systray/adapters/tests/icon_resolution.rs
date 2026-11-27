use super::fixtures::{TempIconDir, first_pixel};
use crate::features::systray::adapters::icon_resolver::{
    ICON_CACHE, InMemorySystrayIconCache, resolve_icon, resolve_pixmap_data,
};
use crate::features::systray::domain::{IconCacheKey, IconImage, IconName, IconThemePath};
use crate::features::systray::ports::{CachedIcon, SystrayIconCachePort};
use crate::shared::primitives::geometry::Size;

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
    cache.insert(key.clone(), CachedIcon::Image(icon_img.clone()));

    assert_eq!(cache.get(&key), Some(CachedIcon::Image(icon_img)));
}

#[test]
fn test_resolve_icon_dynamic_pixmap_updates_not_stale() {
    crate::test_utils::block_on(async {
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
    });
}

#[test]
fn test_resolve_icon_negative_cache_allows_pixmap_fallback() {
    crate::test_utils::block_on(async {
        let icon_name = Some("nonexistent-theme-icon-67890".to_string());
        let icon_none = resolve_icon(icon_name.clone(), None, None).await;
        assert!(icon_none.is_none());

        let key = IconCacheKey::new(IconName::new("nonexistent-theme-icon-67890"), None);
        assert_eq!(ICON_CACHE.get(&key), Some(CachedIcon::Unresolved));

        let pixmap = vec![(1, 1, vec![255, 5, 15, 25])];
        let icon_with_pixmap = resolve_icon(icon_name, None, Some(pixmap)).await;
        assert!(icon_with_pixmap.is_some());
        assert_eq!(icon_with_pixmap.unwrap().data(), &[5, 15, 25, 255]);
    });
}

#[test]
fn test_resolve_icon_empty_string_name_handled() {
    crate::test_utils::block_on(async {
        let pixmap = vec![(1, 1, vec![255, 1, 2, 3])];
        let icon = resolve_icon(Some("   ".to_string()), Some(String::new()), Some(pixmap)).await;
        assert!(icon.is_some());
        assert_eq!(icon.unwrap().data(), &[1, 2, 3, 255]);

        let empty_key = IconCacheKey::new(IconName::new(""), None);
        assert_eq!(ICON_CACHE.get(&empty_key), None);
    });
}

#[test]
fn test_resolve_pixmap_data_picks_the_size_closest_to_the_scaled_target() {
    // The target is 24 * 3 = 72 pixels; 70 is closer than 16 and 80.
    let pixmaps = [
        (16, 1, vec![0; 64]),
        (70, 1, vec![255; 280]),
        (80, 1, vec![0; 320]),
    ];
    let image = resolve_pixmap_data(&pixmaps, 3.0).unwrap();
    assert_eq!(image.size(), &Size::new(70, 1));
    // At scale 1 the target is 24, so 16 wins.
    let image = resolve_pixmap_data(&pixmaps, 1.0).unwrap();
    assert_eq!(image.size(), &Size::new(16, 1));
}

#[test]
fn test_resolve_pixmap_data_keeps_the_first_of_two_equally_close_sizes() {
    let pixmaps = [(70, 1, vec![1; 280]), (74, 1, vec![2; 296])];
    let image = resolve_pixmap_data(&pixmaps, 3.0).unwrap();
    assert_eq!(image.size(), &Size::new(70, 1));
}

#[test]
fn test_resolve_pixmap_data_converts_argb_to_rgba() {
    let pixmaps = [(2, 1, vec![1, 2, 3, 4, 5, 6, 7, 8])];
    let image = resolve_pixmap_data(&pixmaps, 3.0).unwrap();
    assert_eq!(image.data(), &[2, 3, 4, 1, 6, 7, 8, 5]);
    assert_eq!(image.size(), &Size::new(2, 1));
}

#[test]
fn test_resolve_pixmap_data_rejects_the_closest_pixmap_when_malformed() {
    // The closest pixmap has the wrong length; the valid one further away is not tried.
    let pixmaps = [(72, 72, vec![0; 4]), (1, 1, vec![0; 4])];
    assert_eq!(resolve_pixmap_data(&pixmaps, 3.0), None);
    assert_eq!(resolve_pixmap_data(&[(-1, 1, vec![])], 3.0), None);
    assert_eq!(resolve_pixmap_data(&[(1, -1, vec![])], 3.0), None);
    assert_eq!(resolve_pixmap_data(&[], 3.0), None);
}

#[test]
fn test_resolve_icon_prefers_the_theme_png_over_the_svg() {
    crate::test_utils::block_on(async {
        let dir = TempIconDir::new("png-over-svg");
        dir.png("cranky-png-over-svg", [0, 255, 0]);
        dir.svg("cranky-png-over-svg", "#ff0000");
        let name = Some(" cranky-png-over-svg ".to_string());
        let theme = Some(format!(" {} ", dir.theme_path()));
        let image = resolve_icon(name, theme, None).await.unwrap();
        assert_eq!(first_pixel(&image), &[0, 255, 0, 255]);
        assert_eq!(image.size(), &Size::new(72, 72));
        // Cached under the trimmed name and theme path.
        let key = IconCacheKey::new(
            IconName::new("cranky-png-over-svg"),
            Some(IconThemePath::new(dir.theme_path())),
        );
        assert_eq!(ICON_CACHE.get(&key), Some(CachedIcon::Image(image)));
    });
}

#[test]
fn test_resolve_icon_falls_back_to_the_theme_svg() {
    crate::test_utils::block_on(async {
        let dir = TempIconDir::new("svg-only");
        dir.svg("cranky-svg-only", "#0000ff");
        let image = resolve_icon(
            Some("cranky-svg-only".to_string()),
            Some(dir.theme_path()),
            Some(vec![(1, 1, vec![255, 9, 9, 9])]),
        )
        .await
        .unwrap();
        // The disk icon wins over the pixmap.
        assert_eq!(first_pixel(&image), &[0, 0, 255, 255]);
    });
}

#[test]
fn test_resolve_icon_loads_an_absolute_icon_path() {
    crate::test_utils::block_on(async {
        let dir = TempIconDir::new("absolute");
        let path = dir.svg("cranky-absolute", "#ff0000");
        // The freedesktop lookup would take the PNG next to it; the exact path wins.
        dir.png("cranky-absolute", [0, 255, 0]);
        // The theme directory has no such file, so the name is tried as a path.
        let other = TempIconDir::new("absolute-theme");
        let image = resolve_icon(Some(path), Some(other.theme_path()), None)
            .await
            .unwrap();
        assert_eq!(first_pixel(&image), &[255, 0, 0, 255]);
    });
}

#[test]
fn test_resolve_icon_without_name_uses_the_pixmap_and_caches_nothing() {
    crate::test_utils::block_on(async {
        let dir = TempIconDir::new("no-name");
        let pixmap = vec![(1, 1, vec![255, 7, 8, 9])];
        let image = resolve_icon(None, Some(dir.theme_path()), Some(pixmap))
            .await
            .unwrap();
        assert_eq!(image.data(), &[7, 8, 9, 255]);
        assert_eq!(resolve_icon(None, None, Some(vec![])).await, None);
        assert_eq!(resolve_icon(None, None, None).await, None);
    });
}
