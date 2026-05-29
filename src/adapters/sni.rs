use async_trait::async_trait;
use crate::domain::signals::SignalHub;
use crate::domain::applets::{AppletItem, AppletStatus, AppletsState};
use crate::ports::sni::SniPort;
use crate::domain::errors::PortError;
use zbus::{Connection, interface, MessageStream};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error, debug};
use zbus::zvariant::ObjectPath;
use zbus::fdo::PropertiesProxy;
use zbus::names::InterfaceName;
use tokio_stream::StreamExt;
use std::collections::HashMap;
use freedesktop_icons::lookup;

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
    async fn register_status_notifier_item(&self, service: String, #[zbus(header)] header: zbus::message::Header<'_>) {
        debug!("Registered SNI item: {}", service);
        
        let mut full_path = service.clone();
        if !full_path.starts_with('/') {
            full_path = "/StatusNotifierItem".to_string();
        }

        let dbus_dest = if service.starts_with('/') {
            header.sender().map(|s| s.as_str().to_string()).unwrap_or_else(|| service.clone())
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
    async fn track_item(
        conn: Connection,
        items: Arc<RwLock<HashMap<String, AppletItem>>>,
        hub: Arc<SignalHub>,
        dest: String,
        path_str: String,
    ) -> zbus::Result<()> {
        let path = ObjectPath::try_from(path_str.as_str())?;
        
        let props = PropertiesProxy::builder(&conn)
            .destination(dest.clone())?
            .path(path.clone())?
            .build()
            .await?;

        let id = format!("{}{}", dest, path_str);
        
        let iface = InterfaceName::try_from("org.kde.StatusNotifierItem")?;
        let title: String = props.get(iface.clone(), "Title").await.ok().and_then(|v| v.try_into().ok()).unwrap_or_default();
        let status_str: String = props.get(iface.clone(), "Status").await.ok().and_then(|v| v.try_into().ok()).unwrap_or_default();
        let icon_name: Option<String> = props.get(iface.clone(), "IconName").await.ok().and_then(|v| v.try_into().ok());
        
        let status = match status_str.as_str() {
            "Active" => AppletStatus::Active,
            "Passive" => AppletStatus::Passive,
            "NeedsAttention" => AppletStatus::NeedsAttention,
            _ => AppletStatus::Unknown,
        };

        // Try to load icon pixmap if icon_name doesn't exist or as fallback
        // IconPixmap is a(iiay)
        
        let mut applet = AppletItem {
            id: id.clone(),
            destination: dest.clone(),
            path: path_str.clone(),
            title,
            status,
            icon_name: icon_name.clone(),
            icon_data: None,
            icon_width: 0,
            icon_height: 0,
            menu_path: None,
        };

        let max_scale = 3.0f32; // Default to 3.0 for sharp scaling on any screen
        let mut icon_loaded = false;

        // 1. Try to load from IconPixmap first, as many apps (like Slack/Discord) only supply this
        let icon_pixmap: Option<Vec<(i32, i32, Vec<u8>)>> = props.get(iface.clone(), "IconPixmap").await.ok().and_then(|v| v.try_into().ok());
        if let Some(pixmaps) = &icon_pixmap {
            if !pixmaps.is_empty() {
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
                        applet.icon_data = Some(rgba_data);
                        applet.icon_width = w;
                        applet.icon_height = h;
                        icon_loaded = true;
                    }
                }
            }
        }

        // 2. Fall back to IconName if not loaded or if IconPixmap was empty
        if !icon_loaded {
            if let Some(name) = &icon_name {
                if let Some(icon_path) = lookup(name).find() {
                    if let Some(rgba) = crate::utils::load_icon_rgba(&icon_path, 24, max_scale) {
                        applet.icon_width = rgba.width();
                        applet.icon_height = rgba.height();
                        applet.icon_data = Some(rgba.into_raw());
                    }
                }
            }
        }

        {
            let mut lock = items.write().await;
            lock.insert(id.clone(), applet);
        }
        Self::publish_state(&items, &hub).await;

        Ok(())
    }

    async fn publish_state(items: &Arc<RwLock<HashMap<String, AppletItem>>>, hub: &Arc<SignalHub>) {
        let lock = items.read().await;
        let state = AppletsState {
            items: lock.values().cloned().collect(),
        };
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
    async fn start(&mut self) -> Result<(), PortError> {
        let conn = Connection::session().await
            .map_err(|e| PortError::InitFailed(e.to_string()))?;
        
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
                let _res: bool = conn.object_server().at::<&str, Watcher>("/StatusNotifierWatcher", watcher).await
                    .map_err(|e: zbus::Error| PortError::InitFailed(e.to_string()))?;
            },
            Err(_) => {
                info!("Could not claim org.kde.StatusNotifierWatcher. Will attempt to run as host only.");
            }
        }

        *self.conn.lock().await = Some(conn);
        Ok(())
    }

    async fn trigger_action(&self, id: &str, action: &str) -> Result<(), PortError> {
        let lock = self.conn.lock().await;
        let items_lock = self.items.read().await;
        
        if let (Some(conn), Some(applet)) = (lock.as_ref(), items_lock.get(id)) {
            let proxy = zbus::Proxy::new(
                conn,
                applet.destination.clone(),
                applet.path.clone(),
                "org.kde.StatusNotifierItem",
            )
            .await
            .map_err(|e: zbus::Error| PortError::Internal(e.to_string()))?;

            let x: i32 = 0;
            let y: i32 = 0;

            match action {
                "Activate" => {
                    let _ = proxy.call_method("Activate", &(x, y)).await;
                }
                "SecondaryActivate" => {
                    let _ = proxy.call_method("SecondaryActivate", &(x, y)).await;
                }
                "ContextMenu" => {
                    let _ = proxy.call_method("ContextMenu", &(x, y)).await;
                }
                _ => {}
            }
        }
        Ok(())
    }
}
