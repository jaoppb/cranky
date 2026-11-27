//! Turns an item's icon name, theme path or pixmaps into RGBA image data, caching name lookups
//! for the life of the process, including the ones that found nothing.
use crate::features::systray::domain::{IconCacheKey, IconImage, IconName, IconThemePath};
use crate::features::systray::ports::{CachedIcon, SystrayIconCachePort};
use freedesktop_icons::lookup;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

/// Edge of the tray icons in logical pixels, before the scale.
const ICON_SIZE: u16 = 24;

/// The scale icons are resolved at, so they stay sharp on any screen.
const MAX_SCALE: f32 = 3.0;

/// The pixmap whose width is closest to `target_size`; the first of equally close ones.
fn closest_pixmap(
    pixmaps: &[(i32, i32, Vec<u8>)],
    target_size: i32,
) -> Option<&(i32, i32, Vec<u8>)> {
    pixmaps
        .iter()
        .min_by_key(|pixmap| pixmap.0.saturating_sub(target_size).abs())
}

/// The pixmap's ARGB32 bytes as RGBA, or `None` when its size is negative or does not match its
/// data.
fn argb_to_rgba(pixmap: &(i32, i32, Vec<u8>)) -> Option<IconImage> {
    let width = u32::try_from(pixmap.0).ok()?;
    let height = u32::try_from(pixmap.1).ok()?;
    let data = &pixmap.2;
    let expected_len = usize::try_from(width.checked_mul(height)?.checked_mul(4)?).ok()?;
    if data.len() != expected_len {
        return None;
    }
    let rgba_data: Vec<u8> = data
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|&[alpha, red, green, blue]| [red, green, blue, alpha])
        .collect();
    Some(IconImage::new(
        rgba_data,
        crate::shared::primitives::geometry::Size::new(width, height),
    ))
}

/// The pixmap closest to the icon size at `max_scale`, as RGBA; `None` for no pixmap or when the
/// closest one is malformed (the others are not tried).
#[must_use]
pub(super) fn resolve_pixmap_data(
    pixmaps: &[(i32, i32, Vec<u8>)],
    max_scale: f32,
) -> Option<IconImage> {
    let target_size = crate::utils::f32_to_i32((f32::from(ICON_SIZE) * max_scale).round());
    closest_pixmap(pixmaps, target_size).and_then(argb_to_rgba)
}

#[derive(Debug, Default)]
pub(crate) struct InMemorySystrayIconCache(Mutex<HashMap<IconCacheKey, CachedIcon>>);

impl InMemorySystrayIconCache {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
}

impl SystrayIconCachePort for InMemorySystrayIconCache {
    fn get(&self, key: &IconCacheKey) -> Option<CachedIcon> {
        self.0.lock().ok().and_then(|cache| cache.get(key).cloned())
    }

    fn insert(&self, key: IconCacheKey, icon: CachedIcon) {
        if let Ok(mut cache) = self.0.lock() {
            cache.insert(key, icon);
        }
    }
}

pub(super) static ICON_CACHE: LazyLock<InMemorySystrayIconCache> =
    LazyLock::new(InMemorySystrayIconCache::new);

/// `value` trimmed, or `None` when nothing is left.
fn non_blank(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

/// `name.png` in `theme_path`, else `name.svg` there, when either exists.
fn find_in_theme(name: &str, theme_path: &str) -> Option<PathBuf> {
    let base = Path::new(theme_path);
    let png = base.join(format!("{name}.png"));
    if png.exists() {
        return Some(png);
    }
    let svg = base.join(format!("{name}.svg"));
    svg.exists().then_some(svg)
}

/// The file of icon `name`: in the item's theme path first, then `name` itself when it is an
/// existing absolute path, then the freedesktop icon themes.
fn find_icon_path(name: &str, theme_path: Option<&str>) -> Option<PathBuf> {
    if let Some(found) = theme_path.and_then(|theme_path| find_in_theme(name, theme_path)) {
        return Some(found);
    }
    let name_path = Path::new(name);
    if name_path.is_absolute() && name_path.exists() {
        Some(name_path.to_path_buf())
    } else {
        lookup(name).find()
    }
}

/// Finds and decodes icon `name`; blocking, as it reads the disk.
fn load_disk_icon(name: &str, theme_path: Option<&str>) -> Option<IconImage> {
    let icon_path = find_icon_path(name, theme_path)?;
    let (width, height, bytes) = crate::utils::load_icon_rgba(&icon_path, ICON_SIZE, MAX_SCALE)?;
    Some(IconImage::new(
        bytes,
        crate::shared::primitives::geometry::Size::new(width, height),
    ))
}

/// Icon `name` from the cache, or loaded from disk on a blocking thread and cached, a miss
/// included: a lookup that found nothing is not retried.
async fn cached_disk_icon(name: String, theme_path: Option<String>) -> Option<IconImage> {
    let key = IconCacheKey::new(
        IconName::new(name.clone()),
        theme_path.clone().map(IconThemePath::new),
    );
    if let Some(cached) = ICON_CACHE.get(&key) {
        return cached.into_image();
    }
    let loaded_disk_icon =
        tokio::task::spawn_blocking(move || load_disk_icon(&name, theme_path.as_deref()))
            .await
            .unwrap_or(None);
    ICON_CACHE.insert(key, CachedIcon::from(loaded_disk_icon.clone()));
    loaded_disk_icon
}

/// The image of an item's icon: its name looked up on disk (cached), else its pixmaps (never
/// cached, as they change with the item). Names and theme paths are trimmed; a blank one is
/// absent.
pub(super) async fn resolve_icon(
    icon_name: Option<String>,
    icon_theme_path: Option<String>,
    icon_pixmap: Option<Vec<(i32, i32, Vec<u8>)>>,
) -> Option<IconImage> {
    let disk_icon = match non_blank(icon_name) {
        Some(name) => cached_disk_icon(name, non_blank(icon_theme_path)).await,
        None => None,
    };
    disk_icon.or_else(|| {
        icon_pixmap
            .as_deref()
            .and_then(|pixmaps| resolve_pixmap_data(pixmaps, MAX_SCALE))
    })
}
