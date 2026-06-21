use crate::domain::applets::{AppletItem, AppletStatus, AppletsState};
use crate::domain::signals::SignalHub;
use crate::ports::sni::SniPort;
use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SniAdapterError {
    #[error("SNI initialization failed: {0}")]
    InitFailed(String),
    #[error("Internal SNI error: {0}")]
    Internal(String),
}

use freedesktop_icons::lookup;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_stream::StreamExt;
use tracing::{debug, error, info};
use zbus::fdo::PropertiesProxy;
use zbus::names::InterfaceName;
use zbus::zvariant::ObjectPath;
use zbus::{Connection, interface};

#[zbus::proxy(interface = "org.kde.StatusNotifierItem", assume_defaults = true)]
trait StatusNotifierItem {
    #[zbus(property)]
    fn title(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn status(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn icon_name(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn icon_theme_path(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn icon_pixmap(&self) -> zbus::Result<Vec<(i32, i32, Vec<u8>)>>;

    #[zbus(signal)]
    fn new_title(&self) -> zbus::Result<()>;
    #[zbus(signal)]
    fn new_icon(&self) -> zbus::Result<()>;
    #[zbus(signal)]
    fn new_status(&self, status: String) -> zbus::Result<()>;
    #[zbus(signal)]
    fn new_icon_theme_path(&self, path: String) -> zbus::Result<()>;
}

#[derive(Clone)]
pub struct SniAdapter {
    hub: Arc<SignalHub>,
    conn: Arc<tokio::sync::Mutex<Option<Connection>>>,
    items: Arc<RwLock<HashMap<String, AppletItem>>>,
}

struct Watcher {
    items: Arc<RwLock<HashMap<String, AppletItem>>>,
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
        items.keys().cloned().collect()
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
                    crate::domain::applets::AppletId::new(id.clone()),
                    crate::domain::applets::Destination::new(dest.clone()),
                    crate::domain::applets::ObjectPath::new(path_str.clone()),
                    crate::domain::applets::Title::new(String::new()),
                    AppletStatus::Unknown,
                    None,
                    None,
                    None,
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

        let max_scale = 3.0f32; // Default to 3.0 for sharp scaling on any screen

        let icon_pixmap: Option<Vec<(i32, i32, Vec<u8>)>> = all_props
            .remove("IconPixmap")
            .and_then(|v| v.try_into().ok());
        let icon_name_clone = icon_name.clone();
        let (_, icon_image) = tokio::task::spawn_blocking(move || {
            let mut icon_loaded = false;
            let mut icon_image = None;

            // 1. Try to load from IconPixmap first, as many apps (like Slack/Discord) only supply this
            if let Some(pixmaps) = &icon_pixmap
                && !pixmaps.is_empty()
            {
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
                        icon_image = Some(crate::domain::applets::IconImage::new(
                            rgba_data,
                            crate::domain::shared::geometry::Size::new(w, h),
                        ));
                        icon_loaded = true;
                    }
                }
            }

            // 2. Fall back to IconName if not loaded or if IconPixmap was empty
            if !icon_loaded && let Some(name) = &icon_name_clone {
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
                    found_path = lookup(name).find();
                }

                if let Some(icon_path) = found_path
                    && let Some((w, h, bytes)) =
                        crate::utils::load_icon_rgba(&icon_path, 24, max_scale)
                {
                    icon_image = Some(crate::domain::applets::IconImage::new(
                        bytes,
                        crate::domain::shared::geometry::Size::new(w, h),
                    ));
                }
            }

            (icon_loaded, icon_image)
        })
        .await
        .unwrap_or((false, None));

        AppletItem::new(
            crate::domain::applets::AppletId::new(id),
            crate::domain::applets::Destination::new(dest),
            crate::domain::applets::ObjectPath::new(path_str),
            crate::domain::applets::Title::new(title),
            status,
            icon_name.map(crate::domain::applets::IconName::new),
            icon_image,
            None,
        )
    }

    #[tracing::instrument(skip(conn, items, hub))]
    async fn track_item(
        conn: Connection,
        items: Arc<RwLock<HashMap<String, AppletItem>>>,
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
            lock.insert(id.clone(), applet);
        }
        Self::publish_state(&items, &hub).await;

        tracing::info!("Setting up SNI signal streams for {}", id);
        let Ok(mut new_title) = proxy.receive_new_title().await else {
            tracing::error!("Failed to subscribe to new_title for {}", id);
            return Ok(());
        };
        let Ok(mut new_icon) = proxy.receive_new_icon().await else {
            tracing::error!("Failed to subscribe to new_icon for {}", id);
            return Ok(());
        };
        let Ok(mut new_status) = proxy.receive_new_status().await else {
            tracing::error!("Failed to subscribe to new_status for {}", id);
            return Ok(());
        };
        let Ok(mut new_path) = proxy.receive_new_icon_theme_path().await else {
            tracing::error!("Failed to subscribe to new_icon_theme_path for {}", id);
            return Ok(());
        };

        tracing::info!("Successfully subscribed to all SNI signals for {}", id);

        let items_clone = items.clone();
        let hub_clone = hub.clone();
        let id_clone = id.clone();
        let dest_clone = dest.clone();
        let path_str_clone = path_str.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(_) = new_title.next() => { tracing::debug!("{} emitted NewTitle", id_clone); }
                    Some(_) = new_icon.next() => { tracing::debug!("{} emitted NewIcon", id_clone); }
                    Some(_) = new_status.next() => { tracing::debug!("{} emitted NewStatus", id_clone); }
                    Some(_) = new_path.next() => { tracing::debug!("{} emitted NewIconThemePath", id_clone); }
                    else => {
                        tracing::info!("SNI stream ended for {}, breaking loop", id_clone);
                        break;
                    }
                }

                tracing::debug!("Re-fetching applet properties for {}", id_clone);
                let applet = Self::fetch_applet_item(
                    &conn,
                    id_clone.clone(),
                    dest_clone.clone(),
                    path_str_clone.clone(),
                )
                .await;
                {
                    let mut lock = items_clone.write().await;
                    lock.insert(id_clone.clone(), applet);
                }
                Self::publish_state(&items_clone, &hub_clone).await;
            }

            tracing::info!("Applet {} loop terminated, cleaning up state", id_clone);
            {
                let mut lock = items_clone.write().await;
                lock.remove(&id_clone);
            }
            Self::publish_state(&items_clone, &hub_clone).await;
        });

        Ok(())
    }

    async fn publish_state(items: &Arc<RwLock<HashMap<String, AppletItem>>>, hub: &Arc<SignalHub>) {
        let lock = items.read().await;
        let state = AppletsState::new(lock.values().cloned().collect());
        let _ = hub.applets_tx().send(state);
    }
}

impl SniAdapter {
    pub fn new(hub: Arc<SignalHub>) -> Self {
        Self {
            hub,
            conn: Arc::new(tokio::sync::Mutex::new(None)),
            items: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl SniPort for SniAdapter {
    async fn start(&mut self) -> Result<(), SniAdapterError> {
        let conn = Connection::session()
            .await
            .map_err(|e| SniAdapterError::InitFailed(e.to_string()))?;

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
                    .map_err(|e: zbus::Error| SniAdapterError::InitFailed(e.to_string()))?;
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
    async fn trigger_action(&self, id: &str, action: &str) -> Result<(), SniAdapterError> {
        let lock = self.conn.lock().await;
        let items_lock = self.items.read().await;

        if let (Some(conn), Some(applet)) = (lock.as_ref(), items_lock.get(id)) {
            let proxy = zbus::Proxy::new(
                conn,
                applet.destination().as_str().to_string(),
                applet.path().as_str().to_string(),
                "org.kde.StatusNotifierItem",
            )
            .await
            .map_err(|e: zbus::Error| SniAdapterError::Internal(e.to_string()))?;

            let x: i32 = 0;
            let y: i32 = 0;

            match action {
                "Primary" => {
                    if proxy.call_method("ContextMenu", &(x, y)).await.is_err() {
                        let _ = proxy.call_method("Activate", &(x, y)).await;
                    }
                }
                "Activate" => {
                    let _ = proxy.call_method("Activate", &(x, y)).await;
                }
                "SecondaryActivate" => {
                    let _ = proxy.call_method("SecondaryActivate", &(x, y)).await;
                }
                "ContextMenu" => {
                    let _ = proxy.call_method("ContextMenu", &(x, y)).await;
                }
                "ScrollUp" => {
                    let _ = proxy.call_method("Scroll", &(-1, "vertical")).await;
                }
                "ScrollDown" => {
                    let _ = proxy.call_method("Scroll", &(1, "vertical")).await;
                }
                "ScrollLeft" => {
                    let _ = proxy.call_method("Scroll", &(-1, "horizontal")).await;
                }
                "ScrollRight" => {
                    let _ = proxy.call_method("Scroll", &(1, "horizontal")).await;
                }
                _ => {}
            }
        }
        Ok(())
    }
}
