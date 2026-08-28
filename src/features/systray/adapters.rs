use crate::features::systray::domain::{
    IconCacheKey, IconImage, IconName, IconThemePath, SystrayItem, SystrayState, SystrayStatus,
};
use crate::features::systray::ports::{SniPort, SniPortError, SystrayIconCachePort};
use crate::shared::events::signals::SignalHub;
use async_trait::async_trait;

use freedesktop_icons::lookup;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, LazyLock, Mutex};
use tokio::sync::RwLock;

use tracing::{debug, error, info, warn};
use zbus::fdo::PropertiesProxy;
use zbus::names::InterfaceName;
use zbus::zvariant::ObjectPath;
use zbus::{Connection, interface};

type RawTooltip = (String, Vec<(i32, i32, Vec<u8>)>, String, String);

#[zbus::proxy(interface = "org.kde.StatusNotifierItem", assume_defaults = true)]
trait StatusNotifierItem {
    #[zbus(property(emits_changed_signal = "false"))]
    fn title(&self) -> zbus::Result<String>;
    #[zbus(property(emits_changed_signal = "false"))]
    fn status(&self) -> zbus::Result<String>;
    #[zbus(property(emits_changed_signal = "false"))]
    fn icon_name(&self) -> zbus::Result<String>;
    #[zbus(property(emits_changed_signal = "false"))]
    fn icon_theme_path(&self) -> zbus::Result<String>;
    #[zbus(property(emits_changed_signal = "false"))]
    fn icon_pixmap(&self) -> zbus::Result<Vec<(i32, i32, Vec<u8>)>>;

    #[zbus(property(emits_changed_signal = "false"))]
    fn attention_icon_name(&self) -> zbus::Result<String>;
    #[zbus(property(emits_changed_signal = "false"))]
    fn attention_icon_theme_path(&self) -> zbus::Result<String>;
    #[zbus(property(emits_changed_signal = "false"))]
    fn attention_icon_pixmap(&self) -> zbus::Result<Vec<(i32, i32, Vec<u8>)>>;
    #[zbus(property(emits_changed_signal = "false"))]
    fn overlay_icon_name(&self) -> zbus::Result<String>;
    #[zbus(property(emits_changed_signal = "false"))]
    fn overlay_icon_pixmap(&self) -> zbus::Result<Vec<(i32, i32, Vec<u8>)>>;
    #[zbus(property(emits_changed_signal = "false"))]
    fn item_is_menu(&self) -> zbus::Result<bool>;
    #[zbus(property(emits_changed_signal = "false"))]
    fn menu(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
    #[zbus(property(emits_changed_signal = "false"))]
    fn tool_tip(&self) -> zbus::Result<zbus::zvariant::OwnedValue>;

    #[zbus(signal)]
    fn new_title(&self) -> zbus::Result<()>;
    #[zbus(signal)]
    fn new_icon(&self) -> zbus::Result<()>;
    #[zbus(signal)]
    fn new_status(&self, status: String) -> zbus::Result<()>;
    #[zbus(signal)]
    fn new_icon_theme_path(&self, path: String) -> zbus::Result<()>;
    #[zbus(signal)]
    fn new_attention_icon(&self) -> zbus::Result<()>;
    #[zbus(signal)]
    fn new_overlay_icon(&self) -> zbus::Result<()>;
    #[zbus(signal)]
    fn new_tool_tip(&self) -> zbus::Result<()>;
    #[zbus(signal)]
    fn new_menu(&self) -> zbus::Result<()>;
}

#[derive(Debug)]
enum SniEvent {
    Title,
    Status(String),
    Icon,
    ThemePath,
    AttentionIcon,
    OverlayIcon,
    ToolTip,
    Menu,
}

impl SniEvent {
    async fn apply(self, item: SystrayItem, proxy: &StatusNotifierItemProxy<'_>) -> SystrayItem {
        match self {
            Self::Title => {
                let title = proxy.title().await.unwrap_or_default();
                tracing::trace!("SniEvent::Title: updated title to '{title}'");
                item.with_title(title)
            }
            Self::Status(status_str) => {
                let status = match status_str.as_str() {
                    "Active" => SystrayStatus::Active,
                    "Passive" => SystrayStatus::Passive,
                    "NeedsAttention" => SystrayStatus::NeedsAttention,
                    _ => SystrayStatus::Unknown,
                };
                tracing::trace!("SniEvent::Status: updated status to {status:?}");
                item.with_status(status)
            }
            Self::Icon | Self::ThemePath => {
                let icon_name = proxy.icon_name().await.ok();
                let icon_theme_path = proxy.icon_theme_path().await.ok();
                let icon_pixmap = proxy.icon_pixmap().await.ok();
                let has_pixmap = icon_pixmap.as_ref().is_some_and(|p| !p.is_empty());
                tracing::trace!(
                    "SniEvent::Icon/ThemePath: updated icon_name={icon_name:?}, theme_path={icon_theme_path:?}, has_pixmap={has_pixmap}"
                );
                let icon_image =
                    resolve_icon(icon_name.clone(), icon_theme_path, icon_pixmap).await;
                let icon = crate::features::systray::domain::SystrayIcon::new(
                    icon_name.map(crate::features::systray::domain::IconName::new),
                    icon_image,
                );
                item.with_icon(icon)
            }
            Self::AttentionIcon => {
                let icon_name = proxy.attention_icon_name().await.ok();
                let icon_theme_path = proxy.attention_icon_theme_path().await.ok();
                let icon_pixmap = proxy.attention_icon_pixmap().await.ok();
                tracing::trace!("SniEvent::AttentionIcon: updated attention icon");
                let icon_image =
                    resolve_icon(icon_name.clone(), icon_theme_path, icon_pixmap).await;
                let icon = crate::features::systray::domain::SystrayIcon::new(
                    icon_name.map(crate::features::systray::domain::IconName::new),
                    icon_image,
                );
                item.with_attention_icon(icon)
            }
            Self::OverlayIcon => {
                let icon_name = proxy.overlay_icon_name().await.ok();
                let icon_pixmap = proxy.overlay_icon_pixmap().await.ok();
                tracing::trace!("SniEvent::OverlayIcon: updated overlay icon");
                let icon_image = resolve_icon(icon_name.clone(), None, icon_pixmap).await;
                let icon = crate::features::systray::domain::SystrayIcon::new(
                    icon_name.map(crate::features::systray::domain::IconName::new),
                    icon_image,
                );
                item.with_overlay_icon(icon)
            }
            Self::ToolTip => {
                let tooltip = proxy
                    .tool_tip()
                    .await
                    .ok()
                    .and_then(Watcher::parse_raw_tooltip);
                tracing::trace!("SniEvent::ToolTip: updated tooltip");
                item.with_tooltip(tooltip)
            }
            Self::Menu => {
                let mut item = item;
                if let Ok(menu_path) = proxy.menu().await {
                    item = item.with_menu_path(Some(
                        crate::features::systray::domain::ObjectPath::new(menu_path.as_str()),
                    ));
                }
                if let Ok(item_is_menu_val) = proxy.item_is_menu().await {
                    item = item.with_item_is_menu(
                        crate::features::systray::domain::ItemIsMenu::new(item_is_menu_val),
                    );
                }
                tracing::trace!("SniEvent::Menu: updated menu");
                item
            }
        }
    }
}

fn resolve_pixmap_data(
    pixmaps: &[(i32, i32, Vec<u8>)],
    max_scale: f32,
) -> Option<crate::features::systray::domain::IconImage> {
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
            return Some(crate::features::systray::domain::IconImage::new(
                rgba_data,
                crate::shared::primitives::geometry::Size::new(width, height),
            ));
        }
    }
    None
}

#[derive(Debug, Default)]
pub struct InMemorySystrayIconCache(
    Mutex<HashMap<IconCacheKey, Option<IconImage>>>,
);

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

static ICON_CACHE: LazyLock<InMemorySystrayIconCache> = LazyLock::new(InMemorySystrayIconCache::new);

async fn resolve_icon(
    icon_name: Option<String>,
    icon_theme_path: Option<String>,
    icon_pixmap: Option<Vec<(i32, i32, Vec<u8>)>>,
) -> Option<IconImage> {
    let cache_key = icon_name.as_ref().map(|name| {
        IconCacheKey::new(
            IconName::new(name.clone()),
            icon_theme_path.as_ref().map(|tp| IconThemePath::new(tp.clone())),
        )
    });

    if let Some(ref key) = cache_key
        && let Some(cached) = ICON_CACHE.get(key)
    {
        return cached;
    }

    let max_scale = 3.0f32; // Default to 3.0 for sharp scaling on any screen
    let icon_name_clone = icon_name.clone();
    let theme_path_clone = icon_theme_path.clone();
    let (_, icon_image) = tokio::task::spawn_blocking(move || {
        let mut icon_loaded = false;
        let mut icon_image = None;

        if let Some(name) = &icon_name_clone {
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
                && let Some((w, h, bytes)) = crate::utils::load_icon_rgba(&icon_path, 24, max_scale)
            {
                icon_image = Some(IconImage::new(
                    bytes,
                    crate::shared::primitives::geometry::Size::new(w, h),
                ));
                icon_loaded = true;
            }
        }

        if !icon_loaded
            && let Some(pixmaps) = &icon_pixmap
            && !pixmaps.is_empty()
        {
            icon_image = resolve_pixmap_data(pixmaps, max_scale);
            if icon_image.is_some() {
                icon_loaded = true;
            }
        }

        (icon_loaded, icon_image)
    })
    .await
    .unwrap_or((false, None));

    if let Some(key) = cache_key {
        ICON_CACHE.insert(key, icon_image.clone());
    }

    icon_image
}

#[derive(Clone)]
pub struct SniAdapter {
    hub: Arc<SignalHub>,
    conn: Arc<tokio::sync::Mutex<Option<Connection>>>,
    items: Arc<RwLock<BTreeMap<crate::features::systray::domain::SystrayId, SystrayItem>>>,
}

struct Watcher {
    items: Arc<RwLock<BTreeMap<crate::features::systray::domain::SystrayId, SystrayItem>>>,
    hub: Arc<SignalHub>,
    conn: Connection,
    runtime: tokio::runtime::Handle,
}

#[interface(name = "org.kde.StatusNotifierWatcher")]
impl Watcher {
    #[allow(clippy::unused_async)]
    async fn register_status_notifier_item(
        &self,
        service: String,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) {
        debug!("Registered SNI item: {service}");

        let mut full_path = service.clone();
        if !full_path.starts_with('/') {
            full_path = "/StatusNotifierItem".to_string();
        }

        let dbus_dest = if service.starts_with('/') {
            header
                .sender()
                .map_or_else(|| service.clone(), |s| s.as_str().to_string())
        } else {
            service
        };

        let conn = self.conn.clone();
        let items = self.items.clone();
        let hub = self.hub.clone();

        self.runtime.spawn(async move {
            if let Err(e) = Self::track_item(conn, items, hub, dbus_dest, full_path).await {
                error!("Failed to track SNI item: {e}");
            }
        });
    }

    #[allow(clippy::unused_async)]
    async fn register_status_notifier_host(&self, service: String) {
        debug!("Registered SNI host: {service}");
    }

    #[zbus(property)]
    async fn registered_status_notifier_items(&self) -> Vec<String> {
        let items = self.items.read().await;
        items.keys().map(|id| id.as_str().to_string()).collect()
    }

    #[allow(clippy::unused_self)]
    #[zbus(property)]
    const fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    #[allow(clippy::unused_self)]
    #[zbus(property)]
    const fn protocol_version(&self) -> i32 {
        0
    }
}

impl Watcher {
    fn clean_sni_text(input: &str) -> String {
        let s = input
            .replace("<br>", "\n")
            .replace("<br/>", "\n")
            .replace("<br />", "\n")
            .replace("<BR>", "\n")
            .replace("<BR/>", "\n")
            .replace("<BR />", "\n");
        let mut clean = String::with_capacity(s.len());
        let mut in_tag = false;
        for c in s.chars() {
            if c == '<' {
                in_tag = true;
            } else if c == '>' {
                in_tag = false;
            } else if !in_tag {
                clean.push(c);
            }
        }
        clean
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&#10;", "\n")
            .trim()
            .to_string()
    }

    fn parse_raw_tooltip(
        v: zbus::zvariant::OwnedValue,
    ) -> Option<crate::features::systray::domain::SystrayTooltip> {
        if let Ok((icon_name, pixmap, title, description)) = RawTooltip::try_from(v) {
            let icon_name_opt = if icon_name.is_empty() {
                None
            } else {
                Some(icon_name)
            };
            let icon_img_opt = resolve_pixmap_data(&pixmap, 3.0);
            let tooltip_icon = crate::features::systray::domain::SystrayIcon::new(
                icon_name_opt.map(crate::features::systray::domain::IconName::new),
                icon_img_opt,
            );
            Some(crate::features::systray::domain::SystrayTooltip::new(
                tooltip_icon,
                crate::features::systray::domain::SystrayTooltipTitle::new(Self::clean_sni_text(
                    &title,
                )),
                crate::features::systray::domain::SystrayTooltipDescription::new(
                    Self::clean_sni_text(&description),
                ),
            ))
        } else {
            None
        }
    }

    #[allow(clippy::too_many_lines)]
    async fn fetch_systray_item(
        conn: &Connection,
        id: String,
        dest: String,
        path_str: String,
    ) -> SystrayItem {
        let default_item = || {
            SystrayItem::new(
                crate::features::systray::domain::CreateSystrayItemCommand::new(
                    crate::features::systray::domain::SystrayId::new(id.clone()),
                    crate::features::systray::domain::Destination::new(dest.clone()),
                    crate::features::systray::domain::ObjectPath::new(path_str.clone()),
                    crate::features::systray::domain::Title::new(String::new()),
                    SystrayStatus::Unknown,
                    None,
                    None,
                    crate::features::systray::domain::SystrayCategory::ApplicationStatus,
                    crate::features::systray::domain::ItemIsMenu::new(false),
                ),
            )
        };

        let Ok(iface) = InterfaceName::try_from("org.kde.StatusNotifierItem") else {
            return default_item();
        };
        let Ok(path) = ObjectPath::try_from(path_str.as_str()) else {
            return default_item();
        };

        let Ok(props_builder) = PropertiesProxy::builder(conn).destination(dest.clone()) else {
            return default_item();
        };
        let Ok(props_builder) = props_builder.path(path) else {
            return default_item();
        };
        let Ok(props) = props_builder.build().await else {
            return default_item();
        };

        let mut all_props = props.get_all(iface).await.unwrap_or_default();

        let title: String = all_props
            .remove("Title")
            .and_then(|v| v.try_into().ok())
            .unwrap_or_default();
        let status_str: String = all_props
            .remove("Status")
            .and_then(|v| v.try_into().ok())
            .unwrap_or_default();
        let icon_name: Option<String> =
            all_props.remove("IconName").and_then(|v| v.try_into().ok());
        let icon_theme_path: Option<String> = all_props
            .remove("IconThemePath")
            .and_then(|v| v.try_into().ok());
        let category_str: String = all_props
            .remove("Category")
            .and_then(|v| v.try_into().ok())
            .unwrap_or_default();
        let item_id: Option<String> = all_props.remove("Id").and_then(|v| v.try_into().ok());
        let window_id: Option<u32> = all_props
            .remove("WindowId")
            .and_then(|v| v.try_into().ok())
            .or_else(|| {
                all_props
                    .remove("WindowId")
                    .and_then(|v| v.try_into().ok())
                    .and_then(|id: i32| u32::try_from(id).ok())
            });
        let item_is_menu_val: bool = all_props
            .remove("ItemIsMenu")
            .and_then(|v| v.try_into().ok())
            .unwrap_or_default();
        let menu_path_str: Option<String> = all_props
            .remove("Menu")
            .and_then(|v| v.try_into().ok())
            .or_else(|| {
                all_props.remove("Menu").and_then(|v| {
                    if let zbus::zvariant::Value::ObjectPath(p) = &*v {
                        Some(p.as_str().to_string())
                    } else {
                        None
                    }
                })
            });

        tracing::debug!(
            "SNI fetch [{id}]: title='{title}', status='{status_str}', icon_name='{icon_name:?}', theme_path='{icon_theme_path:?}'"
        );

        let status = match status_str.as_str() {
            "Active" => SystrayStatus::Active,
            "Passive" => SystrayStatus::Passive,
            "NeedsAttention" => SystrayStatus::NeedsAttention,
            _ => SystrayStatus::Unknown,
        };

        let icon_pixmap: Option<Vec<(i32, i32, Vec<u8>)>> = all_props
            .remove("IconPixmap")
            .and_then(|v| v.try_into().ok());
        let icon_image =
            resolve_icon(icon_name.clone(), icon_theme_path.clone(), icon_pixmap).await;
        let icon = crate::features::systray::domain::SystrayIcon::new(
            icon_name.map(crate::features::systray::domain::IconName::new),
            icon_image,
        );

        let attention_icon_name: Option<String> = all_props
            .remove("AttentionIconName")
            .and_then(|v| v.try_into().ok());
        let attention_icon_theme_path: Option<String> = all_props
            .remove("AttentionIconThemePath")
            .and_then(|v| v.try_into().ok());
        let attention_icon_pixmap: Option<Vec<(i32, i32, Vec<u8>)>> = all_props
            .remove("AttentionIconPixmap")
            .and_then(|v| v.try_into().ok());
        let attention_icon_image = resolve_icon(
            attention_icon_name.clone(),
            attention_icon_theme_path,
            attention_icon_pixmap,
        )
        .await;
        let attention_icon = crate::features::systray::domain::SystrayIcon::new(
            attention_icon_name.map(crate::features::systray::domain::IconName::new),
            attention_icon_image,
        );

        let overlay_icon_name: Option<String> = all_props
            .remove("OverlayIconName")
            .and_then(|v| v.try_into().ok());
        let overlay_icon_pixmap: Option<Vec<(i32, i32, Vec<u8>)>> = all_props
            .remove("OverlayIconPixmap")
            .and_then(|v| v.try_into().ok());
        let overlay_icon_image = resolve_icon(
            overlay_icon_name.clone(),
            icon_theme_path,
            overlay_icon_pixmap,
        )
        .await;
        let overlay_icon = crate::features::systray::domain::SystrayIcon::new(
            overlay_icon_name.map(crate::features::systray::domain::IconName::new),
            overlay_icon_image,
        );

        let tooltip: Option<crate::features::systray::domain::SystrayTooltip> = all_props
            .remove("ToolTip")
            .or_else(|| all_props.remove("Tooltip"))
            .and_then(Self::parse_raw_tooltip);

        let cmd = crate::features::systray::domain::CreateSystrayItemCommand::new(
            crate::features::systray::domain::SystrayId::new(id),
            crate::features::systray::domain::Destination::new(dest),
            crate::features::systray::domain::ObjectPath::new(path_str),
            crate::features::systray::domain::Title::new(title),
            status,
            icon,
            menu_path_str.map(crate::features::systray::domain::ObjectPath::new),
            crate::features::systray::domain::SystrayCategory::parse_str(&category_str),
            crate::features::systray::domain::ItemIsMenu::new(item_is_menu_val),
        )
        .with_item_id(item_id.map(crate::features::systray::domain::ItemId::new))
        .with_window_id(window_id.map(crate::features::systray::domain::WindowId::new))
        .with_attention_icon(attention_icon)
        .with_overlay_icon(overlay_icon)
        .with_tooltip(tooltip);

        SystrayItem::new(cmd)
    }

    #[tracing::instrument(skip(conn, items, hub))]
    async fn track_item(
        conn: Connection,
        items: Arc<RwLock<BTreeMap<crate::features::systray::domain::SystrayId, SystrayItem>>>,
        hub: Arc<SignalHub>,
        dest: String,
        path_str: String,
    ) -> zbus::Result<()> {
        let id = format!("{dest}{path_str}");

        let proxy = StatusNotifierItemProxy::builder(&conn)
            .destination(dest.clone())?
            .path(path_str.clone())?
            .build()
            .await?;

        let item =
            Self::fetch_systray_item(&conn, id.clone(), dest.clone(), path_str.clone()).await;

        {
            let mut lock = items.write().await;
            lock.insert(crate::features::systray::domain::SystrayId::new(&id), item);
        }
        Self::publish_state(&items, &hub).await;

        tracing::debug!("Setting up SNI signal streams for {id}");
        let Ok(new_title) = proxy.receive_new_title().await else {
            tracing::error!("Failed to subscribe to new_title for {id}");
            return Ok(());
        };
        let Ok(new_icon) = proxy.receive_new_icon().await else {
            tracing::error!("Failed to subscribe to new_icon for {id}");
            return Ok(());
        };
        let Ok(new_status) = proxy.receive_new_status().await else {
            tracing::error!("Failed to subscribe to new_status for {id}");
            return Ok(());
        };
        let Ok(new_path) = proxy.receive_new_icon_theme_path().await else {
            tracing::error!("Failed to subscribe to new_icon_theme_path for {id}");
            return Ok(());
        };
        let Ok(new_attention_icon) = proxy.receive_new_attention_icon().await else {
            tracing::error!("Failed to subscribe to new_attention_icon for {id}");
            return Ok(());
        };
        let Ok(new_overlay_icon) = proxy.receive_new_overlay_icon().await else {
            tracing::error!("Failed to subscribe to new_overlay_icon for {id}");
            return Ok(());
        };
        let Ok(new_tool_tip) = proxy.receive_new_tool_tip().await else {
            tracing::error!("Failed to subscribe to new_tool_tip for {id}");
            return Ok(());
        };
        let Ok(new_menu) = proxy.receive_new_menu().await else {
            tracing::error!("Failed to subscribe to new_menu for {id}");
            return Ok(());
        };

        tracing::debug!("Successfully subscribed to all SNI signals for {id}");

        let items_clone = items.clone();
        let hub_clone = hub.clone();
        let id_clone = id.clone();

        tokio::spawn(async move {
            use tokio_stream::StreamExt;
            let mut events = new_title
                .map(|_| SniEvent::Title)
                .merge(new_status.map(|sig| {
                    SniEvent::Status(sig.args().map(|a| a.status().clone()).unwrap_or_default())
                }))
                .merge(new_icon.map(|_| SniEvent::Icon))
                .merge(new_path.map(|_| SniEvent::ThemePath))
                .merge(new_attention_icon.map(|_| SniEvent::AttentionIcon))
                .merge(new_overlay_icon.map(|_| SniEvent::OverlayIcon))
                .merge(new_tool_tip.map(|_| SniEvent::ToolTip))
                .merge(new_menu.map(|_| SniEvent::Menu));

            while let Some(event) = events.next().await {
                tracing::trace!("Received SNI event {event:?} for systray item {id_clone}");
                let current_item = {
                    let lock = items_clone.read().await;
                    lock.get(&crate::features::systray::domain::SystrayId::new(&id_clone))
                        .cloned()
                };

                let Some(item) = current_item else {
                    break;
                };

                let updated_item = event.apply(item, &proxy).await;

                {
                    let mut lock = items_clone.write().await;
                    lock.insert(
                        crate::features::systray::domain::SystrayId::new(&id_clone),
                        updated_item,
                    );
                }
                Self::publish_state(&items_clone, &hub_clone).await;
            }

            tracing::debug!("Systray item {id_clone} loop terminated, cleaning up state");
            {
                let mut lock = items_clone.write().await;
                lock.remove(&crate::features::systray::domain::SystrayId::new(&id_clone));
            }
            Self::publish_state(&items_clone, &hub_clone).await;
        });

        Ok(())
    }

    async fn remove_by_destination(
        items: &Arc<RwLock<BTreeMap<crate::features::systray::domain::SystrayId, SystrayItem>>>,
        hub: &Arc<SignalHub>,
        destination: &str,
    ) -> bool {
        let keys_to_remove: Vec<_> = {
            let lock = items.read().await;
            lock.iter()
                .filter_map(|(id, item)| {
                    if item.destination().as_str() == destination {
                        Some(id.clone())
                    } else {
                        None
                    }
                })
                .collect()
        };

        let mut removed = false;
        if !keys_to_remove.is_empty() {
            let mut lock = items.write().await;
            for key in keys_to_remove {
                tracing::debug!(
                    "Removing SNI systray item {} because D-Bus name {destination} disconnected",
                    key.as_str()
                );
                lock.remove(&key);
                removed = true;
            }
            drop(lock);
            Self::publish_state(items, hub).await;
        }
        removed
    }

    async fn publish_state(
        items: &Arc<RwLock<BTreeMap<crate::features::systray::domain::SystrayId, SystrayItem>>>,
        hub: &Arc<SignalHub>,
    ) {
        let state = {
            let lock = items.read().await;
            SystrayState::new(lock.clone())
        };
        let _ = hub.systray_tx().send(state);
    }
}

impl SniAdapter {
    #[must_use]
    pub fn new(hub: Arc<SignalHub>) -> Self {
        Self {
            hub,
            conn: Arc::new(tokio::sync::Mutex::new(None)),
            items: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }
}

#[async_trait]
impl SniPort for SniAdapter {
    async fn start(&mut self) -> Result<(), SniPortError> {
        let conn = Connection::session()
            .await
            .map_err(|e| SniPortError::StartFailed(e.to_string()))?;

        let items_clone = self.items.clone();
        let hub_clone = self.hub.clone();
        let conn_clone = conn.clone();

        tokio::spawn(async move {
            use tokio_stream::StreamExt;
            let Ok(proxy) = zbus::fdo::DBusProxy::new(&conn_clone).await else {
                error!("Failed to create DBusProxy for NameOwnerChanged monitoring");
                return;
            };
            let Ok(mut stream) = proxy.receive_name_owner_changed().await else {
                error!("Failed to subscribe to NameOwnerChanged");
                return;
            };

            while let Some(sig) = stream.next().await {
                if let Ok(args) = sig.args() {
                    let is_unowned = args
                        .new_owner()
                        .as_ref()
                        .is_none_or(|n| n.as_str().is_empty());
                    if is_unowned {
                        Watcher::remove_by_destination(&items_clone, &hub_clone, args.name()).await;
                    }
                }
            }
        });

        // Attempt to request the Watcher name
        match conn.request_name("org.kde.StatusNotifierWatcher").await {
            Ok(()) => {
                info!("Successfully claimed org.kde.StatusNotifierWatcher");
                let watcher = Watcher {
                    items: self.items.clone(),
                    hub: self.hub.clone(),
                    conn: conn.clone(),
                    runtime: tokio::runtime::Handle::current(),
                };
                let _res: bool = conn
                    .object_server()
                    .at::<&str, Watcher>("/StatusNotifierWatcher", watcher)
                    .await
                    .map_err(|e: zbus::Error| SniPortError::StartFailed(e.to_string()))?;
            }
            Err(_) => {
                info!(
                    "Could not claim org.kde.StatusNotifierWatcher. Will attempt to run as host only."
                );
            }
        }

        *self.conn.lock().await = Some(conn);
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn trigger_action(
        &self,
        id: &crate::features::systray::domain::SystrayId,
        action: &crate::features::systray::domain::SystrayActionName,
        pos: Option<crate::shared::primitives::geometry::Position>,
    ) -> Result<(), SniPortError> {
        let (conn_opt, item_opt) = {
            let lock = self.conn.lock().await;
            let items_lock = self.items.read().await;
            (lock.clone(), items_lock.get(id).cloned())
        };

        if let (Some(conn), Some(item)) = (conn_opt.as_ref(), item_opt.as_ref()) {
            debug!(
                "trigger_action: Systray item found [id={}, dest={}, path={}, item_is_menu={}], routing action '{}' at pos {pos:?}",
                id.as_str(),
                item.destination().as_str(),
                item.path().as_str(),
                item.item_is_menu().value(),
                action.as_str()
            );
            let proxy = zbus::Proxy::new(
                conn,
                item.destination().as_str().to_string(),
                item.path().as_str().to_string(),
                "org.kde.StatusNotifierItem",
            )
            .await
            .map_err(|e: zbus::Error| {
                error!(
                    "trigger_action: Failed to create D-Bus proxy for {}: {e}",
                    id.as_str()
                );
                SniPortError::ActionFailed {
                    id: id.as_str().to_string(),
                    error: e.to_string(),
                }
            })?;

            let pos_x = pos.map_or(0, |p| p.x());
            let pos_y = pos.map_or(0, |p| p.y());

            match action.as_str() {
                "Primary" => {
                    if item.item_is_menu().value() {
                        debug!(
                            "trigger_action: item_is_menu=true, calling ContextMenu({pos_x}, {pos_y}) on D-Bus"
                        );
                        match proxy.call_method("ContextMenu", &(pos_x, pos_y)).await {
                            Ok(_) => {
                                debug!("trigger_action: ContextMenu({pos_x}, {pos_y}) succeeded");
                            }
                            Err(e) => {
                                error!("trigger_action: ContextMenu({pos_x}, {pos_y}) failed: {e}");
                            }
                        }
                    } else {
                        debug!(
                            "trigger_action: item_is_menu=false, calling Activate({pos_x}, {pos_y}) on D-Bus"
                        );
                        match proxy.call_method("Activate", &(pos_x, pos_y)).await {
                            Ok(_) => debug!("trigger_action: Activate({pos_x}, {pos_y}) succeeded"),
                            Err(e) => {
                                warn!(
                                    "trigger_action: Activate({pos_x}, {pos_y}) failed: {e}, attempting SecondaryActivate({pos_x}, {pos_y})"
                                );
                                match proxy
                                    .call_method("SecondaryActivate", &(pos_x, pos_y))
                                    .await
                                {
                                    Ok(_) => debug!(
                                        "trigger_action: SecondaryActivate({pos_x}, {pos_y}) succeeded"
                                    ),
                                    Err(e2) => error!(
                                        "trigger_action: SecondaryActivate({pos_x}, {pos_y}) failed: {e2}"
                                    ),
                                }
                            }
                        }
                    }
                }
                "Activate" => {
                    debug!("trigger_action: Calling Activate({pos_x}, {pos_y}) on D-Bus");
                    match proxy.call_method("Activate", &(pos_x, pos_y)).await {
                        Ok(_) => debug!("trigger_action: Activate({pos_x}, {pos_y}) succeeded"),
                        Err(e) => error!("trigger_action: Activate({pos_x}, {pos_y}) failed: {e}"),
                    }
                }
                "SecondaryActivate" => {
                    debug!("trigger_action: Calling SecondaryActivate({pos_x}, {pos_y}) on D-Bus");
                    match proxy
                        .call_method("SecondaryActivate", &(pos_x, pos_y))
                        .await
                    {
                        Ok(_) => {
                            debug!("trigger_action: SecondaryActivate({pos_x}, {pos_y}) succeeded");
                        }
                        Err(e) => error!(
                            "trigger_action: SecondaryActivate({pos_x}, {pos_y}) failed: {e}"
                        ),
                    }
                }
                "ContextMenu" => {
                    debug!("trigger_action: Calling ContextMenu({pos_x}, {pos_y}) on D-Bus");
                    match proxy.call_method("ContextMenu", &(pos_x, pos_y)).await {
                        Ok(_) => debug!("trigger_action: ContextMenu({pos_x}, {pos_y}) succeeded"),
                        Err(e) => {
                            error!("trigger_action: ContextMenu({pos_x}, {pos_y}) failed: {e}");
                        }
                    }
                }
                "ScrollUp" => {
                    debug!("trigger_action: Calling Scroll(-1, 'vertical') on D-Bus");
                    match proxy.call_method("Scroll", &(-1, "vertical")).await {
                        Ok(_) => debug!("trigger_action: Scroll(-1, 'vertical') succeeded"),
                        Err(e) => error!("trigger_action: Scroll(-1, 'vertical') failed: {e}"),
                    }
                }
                "ScrollDown" => {
                    debug!("trigger_action: Calling Scroll(1, 'vertical') on D-Bus");
                    match proxy.call_method("Scroll", &(1, "vertical")).await {
                        Ok(_) => debug!("trigger_action: Scroll(1, 'vertical') succeeded"),
                        Err(e) => error!("trigger_action: Scroll(1, 'vertical') failed: {e}"),
                    }
                }
                "ScrollLeft" => {
                    debug!("trigger_action: Calling Scroll(-1, 'horizontal') on D-Bus");
                    match proxy.call_method("Scroll", &(-1, "horizontal")).await {
                        Ok(_) => debug!("trigger_action: Scroll(-1, 'horizontal') succeeded"),
                        Err(e) => error!("trigger_action: Scroll(-1, 'horizontal') failed: {e}"),
                    }
                }
                "ScrollRight" => {
                    debug!("trigger_action: Calling Scroll(1, 'horizontal') on D-Bus");
                    match proxy.call_method("Scroll", &(1, "horizontal")).await {
                        Ok(_) => debug!("trigger_action: Scroll(1, 'horizontal') succeeded"),
                        Err(e) => error!("trigger_action: Scroll(1, 'horizontal') failed: {e}"),
                    }
                }
                other => {
                    warn!("trigger_action: Unrecognized action '{other}'");
                }
            }
        } else {
            if conn_opt.is_none() {
                error!(
                    "trigger_action: No D-Bus connection available when trying to trigger action '{}' on {}",
                    action.as_str(),
                    id.as_str()
                );
            }
            if item_opt.is_none() {
                error!(
                    "trigger_action: Systray item ID '{}' not found in registry when trying to trigger action '{}'",
                    id.as_str(),
                    action.as_str()
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::systray::domain::{
        CreateSystrayItemCommand, Destination, ItemIsMenu, ObjectPath, SystrayCategory, SystrayId,
        Title,
    };
    use crate::shared::config::domain::Config;

    #[tokio::test]
    async fn test_remove_by_destination_removes_matching_items() {
        let hub = Arc::new(SignalHub::new(Config::default()));
        let mut map = BTreeMap::new();

        let item1 = SystrayItem::new(CreateSystrayItemCommand::new(
            SystrayId::new("app1"),
            Destination::new(":1.42"),
            ObjectPath::new("/StatusNotifierItem"),
            Title::new("App 1"),
            SystrayStatus::Active,
            None,
            None,
            SystrayCategory::ApplicationStatus,
            ItemIsMenu::new(false),
        ));
        let item2 = SystrayItem::new(CreateSystrayItemCommand::new(
            SystrayId::new("app2"),
            Destination::new(":1.42"),
            ObjectPath::new("/StatusNotifierItem2"),
            Title::new("App 1 secondary"),
            SystrayStatus::Active,
            None,
            None,
            SystrayCategory::ApplicationStatus,
            ItemIsMenu::new(false),
        ));
        let item3 = SystrayItem::new(CreateSystrayItemCommand::new(
            SystrayId::new("app3"),
            Destination::new(":1.43"),
            ObjectPath::new("/StatusNotifierItem"),
            Title::new("App 2"),
            SystrayStatus::Active,
            None,
            None,
            SystrayCategory::ApplicationStatus,
            ItemIsMenu::new(false),
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
            SystrayId::new("app1"),
            Destination::new(":1.42"),
            ObjectPath::new("/StatusNotifierItem"),
            Title::new("App 1"),
            SystrayStatus::Active,
            None,
            None,
            SystrayCategory::ApplicationStatus,
            ItemIsMenu::new(false),
        ));
        map.insert(SystrayId::new("app1"), item);

        let items = Arc::new(RwLock::new(map));

        let removed = Watcher::remove_by_destination(&items, &hub, ":1.99").await;
        assert!(!removed);

        assert_eq!(items.read().await.len(), 1);
    }

    #[test]
    fn test_sni_event_variants() {
        let _events = [
            SniEvent::Title,
            SniEvent::Status("Active".to_string()),
            SniEvent::Icon,
            SniEvent::ThemePath,
            SniEvent::AttentionIcon,
            SniEvent::OverlayIcon,
        ];
    }

    #[test]
    fn test_clean_sni_text_strips_html_and_converts_br() {
        let raw = "<b>Ducking ON</b><br/>Audio: &lt;enabled&gt; &amp; active";
        let cleaned = Watcher::clean_sni_text(raw);
        assert_eq!(cleaned, "Ducking ON\nAudio: <enabled> & active");
    }

    #[test]
    fn test_parse_raw_tooltip() {
        use zbus::zvariant::Value;
        let raw_tuple: RawTooltip = (
            "test-icon".to_string(),
            vec![],
            "<b>Title</b>".to_string(),
            "Description<br/>Line 2".to_string(),
        );
        let val = Value::from(raw_tuple).try_into().unwrap();
        let tooltip = Watcher::parse_raw_tooltip(val).expect("Should parse tooltip");
        assert_eq!(tooltip.title().as_str(), "Title");
        assert_eq!(tooltip.description().as_str(), "Description\nLine 2");
    }

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
}
