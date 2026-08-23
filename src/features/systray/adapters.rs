use crate::features::applets::domain::{AppletItem, AppletStatus, AppletsState};
use crate::features::systray::ports::{SniPort, SniPortError};
use crate::shared::events::signals::SignalHub;
use async_trait::async_trait;

use freedesktop_icons::lookup;
use std::collections::BTreeMap;
use std::sync::Arc;
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
    async fn apply<'a>(
        self,
        applet: AppletItem,
        proxy: &StatusNotifierItemProxy<'a>,
    ) -> AppletItem {
        match self {
            SniEvent::Title => {
                let title = proxy.title().await.unwrap_or_default();
                tracing::info!("SniEvent::Title: updated title to '{}'", title);
                applet.with_title(title)
            }
            SniEvent::Status(status_str) => {
                let status = match status_str.as_str() {
                    "Active" => AppletStatus::Active,
                    "Passive" => AppletStatus::Passive,
                    "NeedsAttention" => AppletStatus::NeedsAttention,
                    _ => AppletStatus::Unknown,
                };
                tracing::info!("SniEvent::Status: updated status to {:?}", status);
                applet.with_status(status)
            }
            SniEvent::Icon | SniEvent::ThemePath => {
                let icon_name = proxy.icon_name().await.ok();
                let icon_theme_path = proxy.icon_theme_path().await.ok();
                let icon_pixmap = proxy.icon_pixmap().await.ok();
                tracing::info!(
                    "SniEvent::Icon/ThemePath: updated icon_name={:?}, theme_path={:?}, has_pixmap={}",
                    icon_name,
                    icon_theme_path,
                    icon_pixmap.as_ref().map(|p| !p.is_empty()).unwrap_or(false)
                );
                let icon_image =
                    resolve_icon(icon_name.clone(), icon_theme_path, icon_pixmap).await;
                let icon = crate::features::applets::domain::AppletIcon::new(
                    icon_name.map(crate::features::applets::domain::IconName::new),
                    icon_image,
                );
                applet.with_icon(icon)
            }
            SniEvent::AttentionIcon => {
                let icon_name = proxy.attention_icon_name().await.ok();
                let icon_theme_path = proxy.attention_icon_theme_path().await.ok();
                let icon_pixmap = proxy.attention_icon_pixmap().await.ok();
                tracing::info!("SniEvent::AttentionIcon: updated attention icon");
                let icon_image =
                    resolve_icon(icon_name.clone(), icon_theme_path, icon_pixmap).await;
                let icon = crate::features::applets::domain::AppletIcon::new(
                    icon_name.map(crate::features::applets::domain::IconName::new),
                    icon_image,
                );
                applet.with_attention_icon(icon)
            }
            SniEvent::OverlayIcon => {
                let icon_name = proxy.overlay_icon_name().await.ok();
                let icon_pixmap = proxy.overlay_icon_pixmap().await.ok();
                tracing::info!("SniEvent::OverlayIcon: updated overlay icon");
                let icon_image = resolve_icon(icon_name.clone(), None, icon_pixmap).await;
                let icon = crate::features::applets::domain::AppletIcon::new(
                    icon_name.map(crate::features::applets::domain::IconName::new),
                    icon_image,
                );
                applet.with_overlay_icon(icon)
            }
            SniEvent::ToolTip => {
                let tooltip = match proxy.tool_tip().await {
                    Ok(val) => Watcher::parse_raw_tooltip(val),
                    Err(_) => None,
                };
                tracing::info!("SniEvent::ToolTip: updated tooltip");
                applet.with_tooltip(tooltip)
            }
            SniEvent::Menu => {
                let mut applet = applet;
                if let Ok(menu_path) = proxy.menu().await {
                    applet = applet.with_menu_path(Some(
                        crate::features::applets::domain::ObjectPath::new(menu_path.as_str()),
                    ));
                }
                if let Ok(item_is_menu_val) = proxy.item_is_menu().await {
                    applet = applet.with_item_is_menu(
                        crate::features::applets::domain::ItemIsMenu::new(item_is_menu_val),
                    );
                }
                tracing::info!("SniEvent::Menu: updated menu");
                applet
            }
        }
    }
}

fn resolve_pixmap_data(
    pixmaps: &[(i32, i32, Vec<u8>)],
    max_scale: f32,
) -> Option<crate::features::applets::domain::IconImage> {
    if pixmaps.is_empty() {
        return None;
    }
    let target_size = (24.0 * max_scale) as i32;
    let mut best_diff = i32::MAX;
    let mut best_pixmap: Option<&(i32, i32, Vec<u8>)> = None;
    for pixmap in pixmaps {
        let diff = (pixmap.0 - target_size).abs();
        if diff < best_diff {
            best_diff = diff;
            best_pixmap = Some(pixmap);
        }
    }

    if let Some(pixmap) = best_pixmap {
        let w = pixmap.0 as u32;
        let h = pixmap.1 as u32;
        let data = &pixmap.2;
        if data.len() == (w * h * 4) as usize {
            let mut rgba_data = Vec::with_capacity(data.len());
            for chunk in data.chunks_exact(4) {
                let a = chunk[0];
                let r = chunk[1];
                let g = chunk[2];
                let b = chunk[3];
                rgba_data.push(r);
                rgba_data.push(g);
                rgba_data.push(b);
                rgba_data.push(a);
            }
            return Some(crate::features::applets::domain::IconImage::new(
                rgba_data,
                crate::shared::primitives::geometry::Size::new(w, h),
            ));
        }
    }
    None
}

async fn resolve_icon(
    icon_name: Option<String>,
    icon_theme_path: Option<String>,
    icon_pixmap: Option<Vec<(i32, i32, Vec<u8>)>>,
) -> Option<crate::features::applets::domain::IconImage> {
    let max_scale = 3.0f32; // Default to 3.0 for sharp scaling on any screen
    let icon_name_clone = icon_name.clone();
    let (_, icon_image) = tokio::task::spawn_blocking(move || {
        let mut icon_loaded = false;
        let mut icon_image = None;

        if let Some(name) = &icon_name_clone {
            let mut found_path = None;

            if let Some(theme_path) = &icon_theme_path {
                let base = std::path::Path::new(theme_path);
                let png = base.join(format!("{}.png", name));
                if png.exists() {
                    found_path = Some(png);
                } else {
                    let svg = base.join(format!("{}.svg", name));
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
                icon_image = Some(crate::features::applets::domain::IconImage::new(
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

    icon_image
}

#[derive(Clone)]
pub struct SniAdapter {
    hub: Arc<SignalHub>,
    conn: Arc<tokio::sync::Mutex<Option<Connection>>>,
    items: Arc<RwLock<BTreeMap<crate::features::applets::domain::AppletId, AppletItem>>>,
}

struct Watcher {
    items: Arc<RwLock<BTreeMap<crate::features::applets::domain::AppletId, AppletItem>>>,
    hub: Arc<SignalHub>,
    conn: Connection,
    runtime: tokio::runtime::Handle,
}

#[interface(name = "org.kde.StatusNotifierWatcher")]
impl Watcher {
    async fn register_status_notifier_item(
        &self,
        service: String,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) {
        debug!("Registered SNI item: {}", service);

        let mut full_path = service.clone();
        if !full_path.starts_with('/') {
            full_path = "/StatusNotifierItem".to_string();
        }

        let dbus_dest = if service.starts_with('/') {
            header
                .sender()
                .map(|s| s.as_str().to_string())
                .unwrap_or_else(|| service.clone())
        } else {
            service.clone()
        };

        let conn = self.conn.clone();
        let items = self.items.clone();
        let hub = self.hub.clone();

        self.runtime.spawn(async move {
            if let Err(e) = Self::track_item(conn, items, hub, dbus_dest, full_path).await {
                error!("Failed to track SNI item: {}", e);
            }
        });
    }

    async fn register_status_notifier_host(&self, service: String) {
        debug!("Registered SNI host: {}", service);
    }

    #[zbus(property)]
    async fn registered_status_notifier_items(&self) -> Vec<String> {
        let items = self.items.read().await;
        items.keys().map(|id| id.as_str().to_string()).collect()
    }

    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
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
    ) -> Option<crate::features::applets::domain::AppletTooltip> {
        if let Ok((icon_name, pixmap, title, description)) = RawTooltip::try_from(v) {
            let icon_name_opt = if icon_name.is_empty() {
                None
            } else {
                Some(icon_name)
            };
            let icon_img_opt = resolve_pixmap_data(&pixmap, 3.0);
            let tooltip_icon = crate::features::applets::domain::AppletIcon::new(
                icon_name_opt.map(crate::features::applets::domain::IconName::new),
                icon_img_opt,
            );
            Some(crate::features::applets::domain::AppletTooltip::new(
                tooltip_icon,
                crate::features::applets::domain::AppletTooltipTitle::new(Self::clean_sni_text(
                    &title,
                )),
                crate::features::applets::domain::AppletTooltipDescription::new(
                    Self::clean_sni_text(&description),
                ),
            ))
        } else {
            None
        }
    }

    async fn fetch_applet_item(
        conn: &Connection,
        id: String,
        dest: String,
        path_str: String,
    ) -> AppletItem {
        let iface = InterfaceName::try_from("org.kde.StatusNotifierItem").unwrap();
        let path = ObjectPath::try_from(path_str.as_str()).unwrap();

        let props = match PropertiesProxy::builder(conn)
            .destination(dest.clone())
            .unwrap()
            .path(path.clone())
            .unwrap()
            .build()
            .await
        {
            Ok(p) => p,
            Err(_) => {
                return AppletItem::new(
                    crate::features::applets::domain::CreateAppletCommand::new(
                        crate::features::applets::domain::AppletId::new(id.clone()),
                        crate::features::applets::domain::Destination::new(dest.clone()),
                        crate::features::applets::domain::ObjectPath::new(path_str.clone()),
                        crate::features::applets::domain::Title::new(String::new()),
                        AppletStatus::Unknown,
                        None,
                        None,
                        crate::features::applets::domain::AppletCategory::ApplicationStatus,
                        crate::features::applets::domain::ItemIsMenu::new(false),
                    ),
                );
            }
        };

        let mut all_props = props.get_all(iface.clone()).await.unwrap_or_default();

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
                    .map(|id: i32| id as u32)
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
            "SNI fetch [{}]: title='{}', status='{}', icon_name='{:?}', theme_path='{:?}'",
            id,
            title,
            status_str,
            icon_name,
            icon_theme_path
        );

        let status = match status_str.as_str() {
            "Active" => AppletStatus::Active,
            "Passive" => AppletStatus::Passive,
            "NeedsAttention" => AppletStatus::NeedsAttention,
            _ => AppletStatus::Unknown,
        };

        let icon_pixmap: Option<Vec<(i32, i32, Vec<u8>)>> = all_props
            .remove("IconPixmap")
            .and_then(|v| v.try_into().ok());
        let icon_image =
            resolve_icon(icon_name.clone(), icon_theme_path.clone(), icon_pixmap).await;
        let icon = crate::features::applets::domain::AppletIcon::new(
            icon_name.map(crate::features::applets::domain::IconName::new),
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
        let attention_icon = crate::features::applets::domain::AppletIcon::new(
            attention_icon_name.map(crate::features::applets::domain::IconName::new),
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
        let overlay_icon = crate::features::applets::domain::AppletIcon::new(
            overlay_icon_name.map(crate::features::applets::domain::IconName::new),
            overlay_icon_image,
        );

        let tooltip: Option<crate::features::applets::domain::AppletTooltip> = all_props
            .remove("ToolTip")
            .or_else(|| all_props.remove("Tooltip"))
            .and_then(Self::parse_raw_tooltip);

        let cmd = crate::features::applets::domain::CreateAppletCommand::new(
            crate::features::applets::domain::AppletId::new(id),
            crate::features::applets::domain::Destination::new(dest),
            crate::features::applets::domain::ObjectPath::new(path_str),
            crate::features::applets::domain::Title::new(title),
            status,
            icon,
            menu_path_str.map(crate::features::applets::domain::ObjectPath::new),
            crate::features::applets::domain::AppletCategory::parse_str(&category_str),
            crate::features::applets::domain::ItemIsMenu::new(item_is_menu_val),
        )
        .with_item_id(item_id.map(crate::features::applets::domain::ItemId::new))
        .with_window_id(window_id.map(crate::features::applets::domain::WindowId::new))
        .with_attention_icon(attention_icon)
        .with_overlay_icon(overlay_icon)
        .with_tooltip(tooltip);

        AppletItem::new(cmd)
    }

    #[tracing::instrument(skip(conn, items, hub))]
    async fn track_item(
        conn: Connection,
        items: Arc<RwLock<BTreeMap<crate::features::applets::domain::AppletId, AppletItem>>>,
        hub: Arc<SignalHub>,
        dest: String,
        path_str: String,
    ) -> zbus::Result<()> {
        let id = format!("{}{}", dest, path_str);

        let proxy = StatusNotifierItemProxy::builder(&conn)
            .destination(dest.clone())?
            .path(path_str.clone())?
            .build()
            .await?;

        let applet =
            Self::fetch_applet_item(&conn, id.clone(), dest.clone(), path_str.clone()).await;

        {
            let mut lock = items.write().await;
            lock.insert(crate::features::applets::domain::AppletId::new(&id), applet);
        }
        Self::publish_state(&items, &hub).await;

        tracing::info!("Setting up SNI signal streams for {}", id);
        let Ok(new_title) = proxy.receive_new_title().await else {
            tracing::error!("Failed to subscribe to new_title for {}", id);
            return Ok(());
        };
        let Ok(new_icon) = proxy.receive_new_icon().await else {
            tracing::error!("Failed to subscribe to new_icon for {}", id);
            return Ok(());
        };
        let Ok(new_status) = proxy.receive_new_status().await else {
            tracing::error!("Failed to subscribe to new_status for {}", id);
            return Ok(());
        };
        let Ok(new_path) = proxy.receive_new_icon_theme_path().await else {
            tracing::error!("Failed to subscribe to new_icon_theme_path for {}", id);
            return Ok(());
        };
        let Ok(new_attention_icon) = proxy.receive_new_attention_icon().await else {
            tracing::error!("Failed to subscribe to new_attention_icon for {}", id);
            return Ok(());
        };
        let Ok(new_overlay_icon) = proxy.receive_new_overlay_icon().await else {
            tracing::error!("Failed to subscribe to new_overlay_icon for {}", id);
            return Ok(());
        };
        let Ok(new_tool_tip) = proxy.receive_new_tool_tip().await else {
            tracing::error!("Failed to subscribe to new_tool_tip for {}", id);
            return Ok(());
        };
        let Ok(new_menu) = proxy.receive_new_menu().await else {
            tracing::error!("Failed to subscribe to new_menu for {}", id);
            return Ok(());
        };

        tracing::info!("Successfully subscribed to all SNI signals for {}", id);

        let items_clone = items.clone();
        let hub_clone = hub.clone();
        let id_clone = id.clone();

        tokio::spawn(async move {
            use tokio_stream::StreamExt;
            let mut events = new_title
                .map(|_| SniEvent::Title)
                .merge(new_status.map(|sig| {
                    SniEvent::Status(
                        sig.args()
                            .map(|a| a.status().to_string())
                            .unwrap_or_default(),
                    )
                }))
                .merge(new_icon.map(|_| SniEvent::Icon))
                .merge(new_path.map(|_| SniEvent::ThemePath))
                .merge(new_attention_icon.map(|_| SniEvent::AttentionIcon))
                .merge(new_overlay_icon.map(|_| SniEvent::OverlayIcon))
                .merge(new_tool_tip.map(|_| SniEvent::ToolTip))
                .merge(new_menu.map(|_| SniEvent::Menu));

            while let Some(event) = events.next().await {
                tracing::info!("Received SNI event {:?} for applet {}", event, id_clone);
                let current_applet = {
                    let lock = items_clone.read().await;
                    lock.get(&crate::features::applets::domain::AppletId::new(&id_clone))
                        .cloned()
                };

                let Some(applet) = current_applet else {
                    break;
                };

                let updated_applet = event.apply(applet, &proxy).await;

                {
                    let mut lock = items_clone.write().await;
                    lock.insert(
                        crate::features::applets::domain::AppletId::new(&id_clone),
                        updated_applet,
                    );
                }
                Self::publish_state(&items_clone, &hub_clone).await;
            }

            tracing::info!("Applet {} loop terminated, cleaning up state", id_clone);
            {
                let mut lock = items_clone.write().await;
                lock.remove(&crate::features::applets::domain::AppletId::new(&id_clone));
            }
            Self::publish_state(&items_clone, &hub_clone).await;
        });

        Ok(())
    }

    async fn remove_by_destination(
        items: &Arc<RwLock<BTreeMap<crate::features::applets::domain::AppletId, AppletItem>>>,
        hub: &Arc<SignalHub>,
        destination: &str,
    ) -> bool {
        let mut removed = false;
        {
            let mut lock = items.write().await;
            let keys_to_remove: Vec<_> = lock
                .iter()
                .filter_map(|(id, item)| {
                    if item.destination().as_str() == destination {
                        Some(id.clone())
                    } else {
                        None
                    }
                })
                .collect();

            for key in keys_to_remove {
                tracing::info!(
                    "Removing SNI applet {} because D-Bus name {} disconnected",
                    key.as_str(),
                    destination
                );
                lock.remove(&key);
                removed = true;
            }
        }
        if removed {
            Self::publish_state(items, hub).await;
        }
        removed
    }

    async fn publish_state(
        items: &Arc<RwLock<BTreeMap<crate::features::applets::domain::AppletId, AppletItem>>>,
        hub: &Arc<SignalHub>,
    ) {
        let lock = items.read().await;
        let state = AppletsState::new(lock.clone());
        let _ = hub.applets_tx().send(state);
    }
}

impl SniAdapter {
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
                    let is_unowned = args.new_owner().is_none()
                        || args
                            .new_owner()
                            .as_ref()
                            .map(|n| n.as_str().is_empty())
                            .unwrap_or(true);
                    if is_unowned {
                        Watcher::remove_by_destination(&items_clone, &hub_clone, args.name()).await;
                    }
                }
            }
        });

        // Attempt to request the Watcher name
        match conn.request_name("org.kde.StatusNotifierWatcher").await {
            Ok(_) => {
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
        id: &crate::features::applets::domain::AppletId,
        action: &crate::features::applets::domain::AppletActionName,
        pos: Option<crate::shared::primitives::geometry::Position>,
    ) -> Result<(), SniPortError> {
        let lock = self.conn.lock().await;
        let items_lock = self.items.read().await;

        if let (Some(conn), Some(applet)) = (lock.as_ref(), items_lock.get(id)) {
            info!(
                "trigger_action: Applet found [id={}, dest={}, path={}, item_is_menu={}], routing action '{}' at pos {:?}",
                id.as_str(),
                applet.destination().as_str(),
                applet.path().as_str(),
                applet.item_is_menu().value(),
                action.as_str(),
                pos
            );
            let proxy = zbus::Proxy::new(
                conn,
                applet.destination().as_str().to_string(),
                applet.path().as_str().to_string(),
                "org.kde.StatusNotifierItem",
            )
            .await
            .map_err(|e: zbus::Error| {
                error!(
                    "trigger_action: Failed to create D-Bus proxy for {}: {}",
                    id.as_str(),
                    e
                );
                SniPortError::ActionFailed {
                    id: id.as_str().to_string(),
                    error: e.to_string(),
                }
            })?;

            let (x, y) = pos.map(|p| (p.x(), p.y())).unwrap_or((0, 0));

            match action.as_str() {
                "Primary" => {
                    if applet.item_is_menu().value() {
                        info!(
                            "trigger_action: item_is_menu=true, calling ContextMenu({}, {}) on D-Bus",
                            x, y
                        );
                        match proxy.call_method("ContextMenu", &(x, y)).await {
                            Ok(_) => info!("trigger_action: ContextMenu({}, {}) succeeded", x, y),
                            Err(e) => {
                                error!("trigger_action: ContextMenu({}, {}) failed: {}", x, y, e)
                            }
                        }
                    } else {
                        info!(
                            "trigger_action: item_is_menu=false, calling Activate({}, {}) on D-Bus",
                            x, y
                        );
                        match proxy.call_method("Activate", &(x, y)).await {
                            Ok(_) => info!("trigger_action: Activate({}, {}) succeeded", x, y),
                            Err(e) => {
                                warn!(
                                    "trigger_action: Activate({}, {}) failed: {}, attempting SecondaryActivate({}, {})",
                                    x, y, e, x, y
                                );
                                match proxy.call_method("SecondaryActivate", &(x, y)).await {
                                    Ok(_) => info!(
                                        "trigger_action: SecondaryActivate({}, {}) succeeded",
                                        x, y
                                    ),
                                    Err(e2) => error!(
                                        "trigger_action: SecondaryActivate({}, {}) failed: {}",
                                        x, y, e2
                                    ),
                                }
                            }
                        }
                    }
                }
                "Activate" => {
                    info!("trigger_action: Calling Activate({}, {}) on D-Bus", x, y);
                    match proxy.call_method("Activate", &(x, y)).await {
                        Ok(_) => info!("trigger_action: Activate({}, {}) succeeded", x, y),
                        Err(e) => error!("trigger_action: Activate({}, {}) failed: {}", x, y, e),
                    }
                }
                "SecondaryActivate" => {
                    info!(
                        "trigger_action: Calling SecondaryActivate({}, {}) on D-Bus",
                        x, y
                    );
                    match proxy.call_method("SecondaryActivate", &(x, y)).await {
                        Ok(_) => info!("trigger_action: SecondaryActivate({}, {}) succeeded", x, y),
                        Err(e) => error!(
                            "trigger_action: SecondaryActivate({}, {}) failed: {}",
                            x, y, e
                        ),
                    }
                }
                "ContextMenu" => {
                    info!("trigger_action: Calling ContextMenu({}, {}) on D-Bus", x, y);
                    match proxy.call_method("ContextMenu", &(x, y)).await {
                        Ok(_) => info!("trigger_action: ContextMenu({}, {}) succeeded", x, y),
                        Err(e) => error!("trigger_action: ContextMenu({}, {}) failed: {}", x, y, e),
                    }
                }
                "ScrollUp" => {
                    info!("trigger_action: Calling Scroll(-1, 'vertical') on D-Bus");
                    match proxy.call_method("Scroll", &(-1, "vertical")).await {
                        Ok(_) => info!("trigger_action: Scroll(-1, 'vertical') succeeded"),
                        Err(e) => error!("trigger_action: Scroll(-1, 'vertical') failed: {}", e),
                    }
                }
                "ScrollDown" => {
                    info!("trigger_action: Calling Scroll(1, 'vertical') on D-Bus");
                    match proxy.call_method("Scroll", &(1, "vertical")).await {
                        Ok(_) => info!("trigger_action: Scroll(1, 'vertical') succeeded"),
                        Err(e) => error!("trigger_action: Scroll(1, 'vertical') failed: {}", e),
                    }
                }
                "ScrollLeft" => {
                    info!("trigger_action: Calling Scroll(-1, 'horizontal') on D-Bus");
                    match proxy.call_method("Scroll", &(-1, "horizontal")).await {
                        Ok(_) => info!("trigger_action: Scroll(-1, 'horizontal') succeeded"),
                        Err(e) => error!("trigger_action: Scroll(-1, 'horizontal') failed: {}", e),
                    }
                }
                "ScrollRight" => {
                    info!("trigger_action: Calling Scroll(1, 'horizontal') on D-Bus");
                    match proxy.call_method("Scroll", &(1, "horizontal")).await {
                        Ok(_) => info!("trigger_action: Scroll(1, 'horizontal') succeeded"),
                        Err(e) => error!("trigger_action: Scroll(1, 'horizontal') failed: {}", e),
                    }
                }
                other => {
                    warn!("trigger_action: Unrecognized action '{}'", other);
                }
            }
        } else {
            if lock.is_none() {
                error!(
                    "trigger_action: No D-Bus connection available when trying to trigger action '{}' on {}",
                    action.as_str(),
                    id.as_str()
                );
            }
            if items_lock.get(id).is_none() {
                error!(
                    "trigger_action: Applet ID '{}' not found in registry when trying to trigger action '{}'",
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
    use crate::features::applets::domain::{
        AppletCategory, AppletId, CreateAppletCommand, Destination, ItemIsMenu, ObjectPath, Title,
    };
    use crate::shared::config::domain::Config;

    #[tokio::test]
    async fn test_remove_by_destination_removes_matching_items() {
        let hub = Arc::new(SignalHub::new(Config::default()));
        let mut map = BTreeMap::new();

        let item1 = AppletItem::new(CreateAppletCommand::new(
            AppletId::new("app1"),
            Destination::new(":1.42"),
            ObjectPath::new("/StatusNotifierItem"),
            Title::new("App 1"),
            AppletStatus::Active,
            None,
            None,
            AppletCategory::ApplicationStatus,
            ItemIsMenu::new(false),
        ));
        let item2 = AppletItem::new(CreateAppletCommand::new(
            AppletId::new("app2"),
            Destination::new(":1.42"),
            ObjectPath::new("/StatusNotifierItem2"),
            Title::new("App 1 secondary"),
            AppletStatus::Active,
            None,
            None,
            AppletCategory::ApplicationStatus,
            ItemIsMenu::new(false),
        ));
        let item3 = AppletItem::new(CreateAppletCommand::new(
            AppletId::new("app3"),
            Destination::new(":1.43"),
            ObjectPath::new("/StatusNotifierItem"),
            Title::new("App 2"),
            AppletStatus::Active,
            None,
            None,
            AppletCategory::ApplicationStatus,
            ItemIsMenu::new(false),
        ));

        map.insert(AppletId::new("app1"), item1);
        map.insert(AppletId::new("app2"), item2);
        map.insert(AppletId::new("app3"), item3);

        let items = Arc::new(RwLock::new(map));

        let removed = Watcher::remove_by_destination(&items, &hub, ":1.42").await;
        assert!(removed);

        let lock = items.read().await;
        assert_eq!(lock.len(), 1);
        assert!(lock.contains_key(&AppletId::new("app3")));
        assert!(!lock.contains_key(&AppletId::new("app1")));
        assert!(!lock.contains_key(&AppletId::new("app2")));

        let state = hub.applets_rx().borrow().clone();
        assert_eq!(state.items().len(), 1);
        assert!(state.items().contains_key(&AppletId::new("app3")));
    }

    #[tokio::test]
    async fn test_remove_by_destination_no_match_returns_false() {
        let hub = Arc::new(SignalHub::new(Config::default()));
        let mut map = BTreeMap::new();

        let item = AppletItem::new(CreateAppletCommand::new(
            AppletId::new("app1"),
            Destination::new(":1.42"),
            ObjectPath::new("/StatusNotifierItem"),
            Title::new("App 1"),
            AppletStatus::Active,
            None,
            None,
            AppletCategory::ApplicationStatus,
            ItemIsMenu::new(false),
        ));
        map.insert(AppletId::new("app1"), item);

        let items = Arc::new(RwLock::new(map));

        let removed = Watcher::remove_by_destination(&items, &hub, ":1.99").await;
        assert!(!removed);

        let lock = items.read().await;
        assert_eq!(lock.len(), 1);
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
}
