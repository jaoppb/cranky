use crate::features::systray::domain::{
    IconCacheKey, IconImage, IconName, IconThemePath,
};
use crate::features::systray::ports::SystrayIconCachePort;
use freedesktop_icons::lookup;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

#[must_use]
pub fn resolve_pixmap_data(
    pixmaps: &[(i32, i32, Vec<u8>)],
    max_scale: f32,
) -> Option<IconImage> {
    if pixmaps.is_empty() {
        return None;
    }
    #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
    let target_size = (24.0f32 * max_scale).round() as i32;
    let mut best_diff = i32::MAX;
    let mut best_pixmap: Option<&(i32, i32, Vec<u8>)> = None;
    for pixmap in pixmaps {
        let diff = pixmap.0.saturating_sub(target_size).abs();
        if diff < best_diff {
            best_diff = diff;
            best_pixmap = Some(pixmap);
        }
    }

    if let Some(pixmap) = best_pixmap {
        let width = u32::try_from(pixmap.0).ok()?;
        let height = u32::try_from(pixmap.1).ok()?;
        let data = &pixmap.2;
        let expected_len = usize::try_from(width.checked_mul(height)?.checked_mul(4)?).ok()?;
        if data.len() == expected_len {
            let mut rgba_data = Vec::with_capacity(data.len());
            for chunk in data.chunks_exact(4) {
                if let &[alpha, red, green, blue] = chunk {
                    rgba_data.push(red);
                    rgba_data.push(green);
                    rgba_data.push(blue);
                    rgba_data.push(alpha);
                }
            }
            return Some(IconImage::new(
                rgba_data,
                crate::shared::primitives::geometry::Size::new(width, height),
            ));
        }
    }
    None
}

#[derive(Debug, Default)]
pub struct InMemorySystrayIconCache(Mutex<HashMap<IconCacheKey, Option<IconImage>>>);

impl InMemorySystrayIconCache {
    #[must_use]
    pub fn new() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
}

impl SystrayIconCachePort for InMemorySystrayIconCache {
    fn get(&self, key: &IconCacheKey) -> Option<Option<IconImage>> {
        self.0.lock().ok().and_then(|cache| cache.get(key).cloned())
    }

    fn insert(&self, key: IconCacheKey, image: Option<IconImage>) {
        if let Ok(mut cache) = self.0.lock() {
            cache.insert(key, image);
        }
    }
}

pub static ICON_CACHE: LazyLock<InMemorySystrayIconCache> =
    LazyLock::new(InMemorySystrayIconCache::new);

pub async fn resolve_icon(
    icon_name: Option<String>,
    icon_theme_path: Option<String>,
    icon_pixmap: Option<Vec<(i32, i32, Vec<u8>)>>,
) -> Option<IconImage> {
    let clean_icon_name = icon_name.and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    let clean_theme_path = icon_theme_path.and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    let cache_key = clean_icon_name.as_ref().map(|name| {
        IconCacheKey::new(
            IconName::new(name.clone()),
            clean_theme_path
                .as_ref()
                .map(|tp| IconThemePath::new(tp.clone())),
        )
    });

    let max_scale = 3.0f32; // Default to 3.0 for sharp scaling on any screen

    // 1. Check icon name cache for disk/theme icons
    let disk_icon = if let Some(ref key) = cache_key {
        if let Some(cached) = ICON_CACHE.get(key) {
            // Cached result: either Some(icon_image) or None (cached negative lookup)
            cached
        } else {
            // Cache miss: resolve disk/theme lookup asynchronously on blocking thread
            let name_clone = clean_icon_name.clone();
            let theme_path_clone = clean_theme_path.clone();
            let loaded_disk_icon = tokio::task::spawn_blocking(move || {
                let Some(name) = &name_clone else {
                    return None;
                };
                let mut found_path = None;

                if let Some(theme_path) = &theme_path_clone {
                    let base = std::path::Path::new(theme_path);
                    let png = base.join(format!("{name}.png"));
                    if png.exists() {
                        found_path = Some(png);
                    } else {
                        let svg = base.join(format!("{name}.svg"));
                        if svg.exists() {
                            found_path = Some(svg);
                        }
                    }
                }

                if found_path.is_none() {
                    let p = std::path::Path::new(name);
                    if p.is_absolute() && p.exists() {
                        found_path = Some(p.to_path_buf());
                    } else {
                        found_path = lookup(name).find();
                    }
                }

                if let Some(icon_path) = found_path
                    && let Some((w, h, bytes)) =
                        crate::utils::load_icon_rgba(&icon_path, 24, max_scale)
                {
                    Some(IconImage::new(
                        bytes,
                        crate::shared::primitives::geometry::Size::new(w, h),
                    ))
                } else {
                    None
                }
            })
            .await
            .unwrap_or(None);

            ICON_CACHE.insert(key.clone(), loaded_disk_icon.clone());
            loaded_disk_icon
        }
    } else {
        None
    };

    if disk_icon.is_some() {
        return disk_icon;
    }

    // 2. If no disk/theme icon was resolved, fallback to dynamic raw pixmap (never cached globally)
    if let Some(ref pixmaps) = icon_pixmap
        && !pixmaps.is_empty()
    {
        return resolve_pixmap_data(pixmaps, max_scale);
    }

    None
}
