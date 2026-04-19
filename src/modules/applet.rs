use crate::modules::{CrankyModule, Event, UpdateAction};
use crate::render::{RenderContext, TextStyling};
use crate::utils::rasterize_svg_icon_rgba;
use log::{debug, warn};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use thiserror::Error;
use tiny_skia::{Color, Paint, PixmapMut, Rect, Transform};
use zbus::message::Header;
use zbus::object_server::SignalEmitter;

const ITEM_SPACING: f32 = 8.0;
const ICON_TEXT_GAP: f32 = 6.0;

#[derive(Error, Debug)]
pub enum AppletError {
    #[error("DBus error: {0}")]
    Dbus(#[from] zbus::Error),
    #[error("DBus FDO error: {0}")]
    DbusFdo(#[from] zbus::fdo::Error),
    #[error("Applet provider error: {0}")]
    Provider(String),
}

type AppletResult<T> = std::result::Result<T, AppletError>;

#[derive(Debug, Deserialize, Clone)]
pub struct AppletConfig {
    #[serde(default = "default_refresh_ms")]
    refresh_ms: u64,
    #[serde(default = "default_show_titles")]
    show_titles: bool,
    #[serde(default = "default_show_icons")]
    show_icons: bool,
    #[serde(default = "default_icon_size")]
    icon_size: u16,
    #[serde(default)]
    icon_theme: Option<String>,
    #[serde(default = "default_max_items")]
    max_items: usize,
    #[serde(default = "default_empty_label")]
    empty_label: String,
}

impl Default for AppletConfig {
    fn default() -> Self {
        Self {
            refresh_ms: default_refresh_ms(),
            show_titles: default_show_titles(),
            show_icons: default_show_icons(),
            icon_size: default_icon_size(),
            icon_theme: None,
            max_items: default_max_items(),
            empty_label: default_empty_label(),
        }
    }
}

impl AppletConfig {
    pub fn refresh_ms(&self) -> u64 {
        self.refresh_ms
    }

    pub fn show_titles(&self) -> bool {
        self.show_titles
    }

    pub fn show_icons(&self) -> bool {
        self.show_icons
    }

    pub fn icon_size(&self) -> u16 {
        self.icon_size
    }

    pub fn icon_theme(&self) -> Option<&str> {
        self.icon_theme.as_deref()
    }

    pub fn max_items(&self) -> usize {
        self.max_items
    }

    pub fn empty_label(&self) -> &str {
        &self.empty_label
    }
}

fn default_refresh_ms() -> u64 {
    1000
}

fn default_show_titles() -> bool {
    true
}

fn default_show_icons() -> bool {
    true
}

fn default_icon_size() -> u16 {
    16
}

fn default_max_items() -> usize {
    6
}

fn default_empty_label() -> String {
    "applet: none".to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AppletItem {
    service_name: String,
    object_path: String,
    title: String,
    app_id: String,
    status: String,
    icon_name: String,
}

#[derive(Debug, Clone)]
struct IconBitmap {
    width: u32,
    height: u32,
    rgba_pixels: Vec<u8>,
}

#[derive(Debug, Clone)]
struct DesktopEntryMeta {
    name: String,
    icon: String,
    exec: String,
    startup_wm_class: String,
    file_stem: String,
}

trait AppletProvider: Send + Sync {
    fn list_items(&self) -> AppletResult<Vec<AppletItem>>;
}

fn normalize_registered_item(
    service_or_path: &str,
    sender: Option<&str>,
) -> Option<(String, String)> {
    let trimmed = service_or_path.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.starts_with('/') {
        let sender = sender?.trim();
        if sender.is_empty() {
            return None;
        }
        return Some((sender.to_string(), trimmed.to_string()));
    }

    if let Some(path_start) = trimmed.find('/') {
        let service = trimmed[..path_start].trim();
        let object_path = trimmed[path_start..].trim();
        if service.is_empty() || object_path.is_empty() {
            return None;
        }
        return Some((service.to_string(), object_path.to_string()));
    }

    Some((trimmed.to_string(), "/StatusNotifierItem".to_string()))
}

#[derive(Clone)]
struct WatcherRegistry {
    items: Arc<Mutex<Vec<String>>>,
}

impl WatcherRegistry {
    fn new() -> Self {
        Self {
            items: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn register(&self, item: &str) -> bool {
        let mut guard = self.items.lock().unwrap();
        if guard.iter().any(|existing| existing == item) {
            return false;
        }
        guard.push(item.to_string());
        true
    }

    fn list(&self) -> Vec<String> {
        self.items.lock().unwrap().clone()
    }
}

struct KdeStatusNotifierWatcher {
    registry: WatcherRegistry,
}

impl KdeStatusNotifierWatcher {
    fn new(registry: WatcherRegistry) -> Self {
        Self { registry }
    }
}

#[zbus::interface(name = "org.kde.StatusNotifierWatcher")]
impl KdeStatusNotifierWatcher {
    async fn register_status_notifier_item(
        &self,
        service_or_path: &str,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        let sender = header.sender().map(|s| s.as_str());
        if let Some((service, object_path)) = normalize_registered_item(service_or_path, sender) {
            let item = format!("{service}{object_path}");
            if self.registry.register(&item) {
                Self::status_notifier_item_registered(&emitter, &item)
                    .await
                    .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
            }
            Ok(())
        } else {
            Err(zbus::fdo::Error::InvalidArgs(
                "invalid StatusNotifierItem registration".to_string(),
            ))
        }
    }

    async fn register_status_notifier_host(&self, _service: &str) -> zbus::fdo::Result<()> {
        Ok(())
    }

    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        self.registry.list()
    }

    #[zbus(signal)]
    async fn status_notifier_item_registered(
        emitter: &SignalEmitter<'_>,
        item: &str,
    ) -> zbus::Result<()>;
}

struct FreedesktopStatusNotifierWatcher {
    registry: WatcherRegistry,
}

impl FreedesktopStatusNotifierWatcher {
    fn new(registry: WatcherRegistry) -> Self {
        Self { registry }
    }
}

#[zbus::interface(name = "org.freedesktop.StatusNotifierWatcher")]
impl FreedesktopStatusNotifierWatcher {
    async fn register_status_notifier_item(
        &self,
        service_or_path: &str,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        let sender = header.sender().map(|s| s.as_str());
        if let Some((service, object_path)) = normalize_registered_item(service_or_path, sender) {
            let item = format!("{service}{object_path}");
            if self.registry.register(&item) {
                Self::status_notifier_item_registered(&emitter, &item)
                    .await
                    .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
            }
            Ok(())
        } else {
            Err(zbus::fdo::Error::InvalidArgs(
                "invalid StatusNotifierItem registration".to_string(),
            ))
        }
    }

    async fn register_status_notifier_host(&self, _service: &str) -> zbus::fdo::Result<()> {
        Ok(())
    }

    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        self.registry.list()
    }

    #[zbus(signal)]
    async fn status_notifier_item_registered(
        emitter: &SignalEmitter<'_>,
        item: &str,
    ) -> zbus::Result<()>;
}

struct LocalWatcherHost {
    connection: zbus::blocking::Connection,
    registry: WatcherRegistry,
    owned_names: Vec<String>,
}

impl LocalWatcherHost {
    fn start() -> AppletResult<Option<Self>> {
        let connection = zbus::blocking::Connection::session()?;
        let mut owned_names = Vec::new();
        for name in [
            "org.kde.StatusNotifierWatcher",
            "org.freedesktop.StatusNotifierWatcher",
            "org.ayatana.StatusNotifierWatcher",
        ] {
            match connection.request_name(name) {
                Ok(()) => owned_names.push(name.to_string()),
                Err(error) => {
                    debug!("Could not own watcher name '{}': {}", name, error);
                }
            }
        }

        if owned_names.is_empty() {
            return Ok(None);
        }

        let registry = WatcherRegistry::new();
        {
            let object_server = connection.object_server();
            object_server.at(
                "/StatusNotifierWatcher",
                KdeStatusNotifierWatcher::new(registry.clone()),
            )?;
            object_server.at(
                "/StatusNotifierWatcher",
                FreedesktopStatusNotifierWatcher::new(registry.clone()),
            )?;
        }

        Ok(Some(Self {
            connection,
            registry,
            owned_names,
        }))
    }

    fn registered_items(&self) -> Vec<(String, String)> {
        self.registry
            .list()
            .into_iter()
            .filter_map(|entry| normalize_registered_item(&entry, None))
            .collect()
    }

    fn is_active(&self) -> bool {
        let _ = self.connection.unique_name();
        !self.owned_names.is_empty()
    }
}

enum LocalWatcherState {
    Uninitialized,
    Active(LocalWatcherHost),
    Unavailable,
}

fn global_watcher_state() -> &'static Mutex<LocalWatcherState> {
    static STATE: OnceLock<Mutex<LocalWatcherState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(LocalWatcherState::Uninitialized))
}

pub(crate) fn drop_global_watcher() {
    let mut guard = global_watcher_state().lock().unwrap();
    *guard = LocalWatcherState::Uninitialized;
}

struct RealAppletProvider;

impl RealAppletProvider {
    fn new() -> Self {
        Self
    }

    fn local_registered_items(&self) -> Vec<(String, String)> {
        let mut guard = global_watcher_state().lock().unwrap();
        match &mut *guard {
            LocalWatcherState::Active(host) => {
                if host.is_active() {
                    host.registered_items()
                } else {
                    *guard = LocalWatcherState::Unavailable;
                    Vec::new()
                }
            }
            LocalWatcherState::Uninitialized => match LocalWatcherHost::start() {
                Ok(Some(host)) => {
                    let items = host.registered_items();
                    *guard = LocalWatcherState::Active(host);
                    items
                }
                Ok(None) => {
                    *guard = LocalWatcherState::Unavailable;
                    Vec::new()
                }
                Err(error) => {
                    warn!("Failed to start local watcher host: {}", error);
                    *guard = LocalWatcherState::Unavailable;
                    Vec::new()
                }
            },
            LocalWatcherState::Unavailable => Vec::new(),
        }
    }

    fn is_status_notifier_item(name: &str) -> bool {
        name.starts_with("org.kde.StatusNotifierItem-")
            || name.starts_with("org.freedesktop.StatusNotifierItem-")
            || name.starts_with("org.ayatana.StatusNotifierItem-")
    }

    fn parse_registered_item(entry: &str) -> Option<(String, String)> {
        normalize_registered_item(entry, None)
    }

    fn watcher_registered_items(connection: &zbus::blocking::Connection) -> Vec<(String, String)> {
        let mut endpoints = Vec::new();
        for watcher_iface in [
            "org.kde.StatusNotifierWatcher",
            "org.freedesktop.StatusNotifierWatcher",
        ] {
            let proxy = match zbus::blocking::Proxy::new(
                connection,
                watcher_iface,
                "/StatusNotifierWatcher",
                watcher_iface,
            ) {
                Ok(proxy) => proxy,
                Err(error) => {
                    debug!("Watcher '{}' unavailable: {}", watcher_iface, error);
                    continue;
                }
            };

            match proxy.get_property::<Vec<String>>("RegisteredStatusNotifierItems") {
                Ok(items) => {
                    for item in items {
                        if let Some(endpoint) = Self::parse_registered_item(&item) {
                            endpoints.push(endpoint);
                        }
                    }
                }
                Err(error) => {
                    debug!(
                        "Failed to read RegisteredStatusNotifierItems from '{}': {}",
                        watcher_iface, error
                    );
                }
            }
        }
        endpoints
    }

    fn read_string_property(
        proxy: &zbus::blocking::Proxy<'_>,
        property: &str,
        service_name: &str,
    ) -> String {
        match proxy.get_property::<String>(property) {
            Ok(value) => value,
            Err(error) => {
                debug!(
                    "Missing or unreadable property '{}' for {}: {}",
                    property, service_name, error
                );
                String::new()
            }
        }
    }

    fn fetch_item(
        connection: &zbus::blocking::Connection,
        service_name: &str,
        object_path: &str,
    ) -> AppletResult<AppletItem> {
        let mut last_err: Option<zbus::Error> = None;
        for iface in [
            "org.kde.StatusNotifierItem",
            "org.freedesktop.StatusNotifierItem",
            "org.ayatana.StatusNotifierItem",
        ] {
            match zbus::blocking::Proxy::new(connection, service_name, object_path, iface) {
                Ok(proxy) => {
                    let mut title = Self::read_string_property(&proxy, "Title", service_name);
                    let app_id = Self::read_string_property(&proxy, "Id", service_name);
                    let status = Self::read_string_property(&proxy, "Status", service_name);
                    let icon_name = Self::read_string_property(&proxy, "IconName", service_name);
                    if title.is_empty()
                        && let Ok((_, _, tooltip_title, _)) =
                            proxy
                                .get_property::<(String, Vec<(i32, i32, Vec<u8>)>, String, String)>(
                                    "ToolTip",
                                )
                    {
                        title = tooltip_title;
                    }

                    return Ok(AppletItem {
                        service_name: service_name.to_string(),
                        object_path: object_path.to_string(),
                        title,
                        app_id,
                        status,
                        icon_name,
                    });
                }
                Err(error) => {
                    last_err = Some(error);
                }
            }
        }

        Err(last_err.map(AppletError::from).unwrap_or_else(|| {
            AppletError::Provider("No StatusNotifierItem interface".to_string())
        }))
    }
}

impl AppletProvider for RealAppletProvider {
    fn list_items(&self) -> AppletResult<Vec<AppletItem>> {
        let connection = zbus::blocking::Connection::session()?;
        let dbus_proxy = zbus::blocking::fdo::DBusProxy::new(&connection)?;
        let mut endpoints: Vec<(String, String)> = self.local_registered_items();
        endpoints.extend(Self::watcher_registered_items(&connection));

        for name in dbus_proxy.list_names()? {
            let service_name = name.as_str();
            if !Self::is_status_notifier_item(service_name) {
                continue;
            }
            endpoints.push((service_name.to_string(), "/StatusNotifierItem".to_string()));
        }

        let mut seen = HashSet::new();
        let mut items = Vec::new();
        for (service_name, object_path) in endpoints {
            if !seen.insert((service_name.clone(), object_path.clone())) {
                continue;
            }

            match Self::fetch_item(&connection, &service_name, &object_path) {
                Ok(item) => items.push(item),
                Err(error) => {
                    warn!(
                        "Failed to read status notifier item '{}{}': {}",
                        service_name, object_path, error
                    );
                }
            }
        }

        items.sort_by(|a, b| a.service_name.cmp(&b.service_name));
        Ok(items)
    }
}

pub struct AppletModule {
    provider: Box<dyn AppletProvider>,
    items: Vec<AppletItem>,
    icon_cache: HashMap<String, Option<IconBitmap>>,
    resolved_icon_by_item: HashMap<String, String>,
    desktop_icon_cache: HashMap<String, Option<String>>,
    desktop_entries: Option<Vec<DesktopEntryMeta>>,
    last_refresh: Option<Instant>,
    refresh_interval: Duration,
    show_titles: bool,
    show_icons: bool,
    icon_size: u16,
    icon_theme: Option<String>,
    max_items: usize,
    empty_label: String,
    font_family: String,
    error_message: Option<String>,
    render_scale_bits: AtomicU32,
    icon_cache_scale: f32,
}

impl AppletModule {
    pub fn new() -> Self {
        Self::with_provider(Box::new(RealAppletProvider::new()))
    }

    fn with_provider(provider: Box<dyn AppletProvider>) -> Self {
        Self {
            provider,
            items: Vec::new(),
            icon_cache: HashMap::new(),
            resolved_icon_by_item: HashMap::new(),
            desktop_icon_cache: HashMap::new(),
            desktop_entries: None,
            last_refresh: None,
            refresh_interval: Duration::from_millis(default_refresh_ms()),
            show_titles: default_show_titles(),
            show_icons: default_show_icons(),
            icon_size: default_icon_size(),
            icon_theme: None,
            max_items: default_max_items(),
            empty_label: default_empty_label(),
            font_family: String::new(),
            error_message: None,
            render_scale_bits: AtomicU32::new(1.0f32.to_bits()),
            icon_cache_scale: 1.0,
        }
    }

    fn should_refresh(&self, now: Instant) -> bool {
        match self.last_refresh {
            Some(last) => now.duration_since(last) >= self.refresh_interval,
            None => true,
        }
    }

    fn visible_count(&self) -> usize {
        self.items.len().min(self.max_items.max(1))
    }

    fn item_title(&self, item: &AppletItem) -> String {
        let base = if !item.title.is_empty() {
            item.title.clone()
        } else if !item.app_id.is_empty() {
            item.app_id.clone()
        } else if !item.icon_name.is_empty() {
            item.icon_name.clone()
        } else {
            item.object_path
                .rsplit('/')
                .find(|part| !part.is_empty() && !part.chars().all(|c| c.is_ascii_digit()))
                .map(|s| s.to_string())
                .filter(|s| s != "StatusNotifierItem")
                .unwrap_or_else(|| item.service_name.clone())
        };

        if item.status.is_empty() || item.status == "Active" {
            base
        } else {
            format!("{} [{}]", base, item.status)
        }
    }

    fn item_id(item: &AppletItem) -> String {
        format!("{}{}", item.service_name, item.object_path)
    }

    fn item_fallback_letter(item: &AppletItem) -> Option<String> {
        let title_char = item.title.chars().find(|c| !c.is_whitespace());
        if let Some(c) = title_char {
            return Some(c.to_uppercase().to_string());
        }
        let id_char = item.app_id.chars().find(|c| !c.is_whitespace());
        if let Some(c) = id_char {
            return Some(c.to_uppercase().to_string());
        }
        let icon_char = item.icon_name.chars().find(|c| !c.is_whitespace());
        if let Some(c) = icon_char {
            return Some(c.to_uppercase().to_string());
        }
        let service_char = item
            .service_name
            .chars()
            .find(|c| c.is_ascii_alphabetic())
            .or_else(|| {
                item.service_name
                    .chars()
                    .find(|c| c.is_ascii_alphanumeric())
            })
            .or_else(|| item.service_name.chars().find(|c| !c.is_whitespace()));
        service_char.map(|c| c.to_uppercase().to_string())
    }

    fn item_descriptor(item: &AppletItem) -> String {
        format!(
            "{}|{}|{}|{}",
            item.service_name, item.title, item.app_id, item.icon_name
        )
    }

    fn render_empty_or_error_label(&self) -> Option<String> {
        if let Some(error) = &self.error_message {
            Some(format!("applet: error ({})", error))
        } else if self.items.is_empty() {
            Some(self.empty_label.clone())
        } else {
            None
        }
    }

    fn text_styling(&self) -> TextStyling {
        TextStyling::new(
            14.0,
            20.0,
            Color::from_rgba8(192, 202, 245, 255),
            self.font_family.clone(),
        )
    }

    fn load_icon_bitmap(&self, icon_key: &str, scale: f32) -> Option<IconBitmap> {
        if icon_key.is_empty() {
            return None;
        }

        let path = if Path::new(icon_key).is_absolute() {
            PathBuf::from(icon_key)
        } else {
            let mut resolved = None;
            for size in Self::icon_lookup_sizes(self.icon_size, scale) {
                let mut lookup = freedesktop_icons::lookup(icon_key)
                    .with_size(size)
                    .with_cache();
                if let Some(theme) = self.icon_theme.as_deref() {
                    lookup = lookup.with_theme(theme);
                }
                if let Some(path) = lookup.find() {
                    resolved = Some(path);
                    break;
                }
            }
            resolved?
        };

        let is_svg = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"));
        let rgba = if is_svg {
            match rasterize_svg_icon_rgba(&path, self.icon_size, scale) {
                Some(pixels) => pixels,
                None => {
                    warn!("Failed to decode SVG icon '{}'", path.display());
                    return None;
                }
            }
        } else {
            let image = match image::ImageReader::open(&path) {
                Ok(reader) => match reader.decode() {
                    Ok(decoded) => decoded,
                    Err(error) => {
                        warn!("Failed to decode icon '{}': {}", path.display(), error);
                        return None;
                    }
                },
                Err(error) => {
                    warn!("Failed to open icon '{}': {}", path.display(), error);
                    return None;
                }
            };
            image.to_rgba8()
        };

        Some(IconBitmap {
            width: rgba.width(),
            height: rgba.height(),
            rgba_pixels: rgba.into_raw(),
        })
    }

    fn icon_lookup_sizes(icon_size: u16, scale: f32) -> Vec<u16> {
        let scaled_size = ((icon_size as f32) * scale.max(1.0)).ceil() as u16;
        let mut sizes = vec![
            scaled_size.max(icon_size),
            icon_size,
            16,
            22,
            24,
            32,
            48,
            64,
            96,
            128,
            256,
        ];
        let mut seen = HashSet::new();
        sizes.retain(|size| *size > 0 && seen.insert(*size));
        sizes
    }

    fn applications_dirs() -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(PathBuf::from(&home).join(".local/share/applications"));
            dirs.push(PathBuf::from(&home).join(".local/share/flatpak/exports/share/applications"));
        }
        dirs.push(PathBuf::from("/usr/share/applications"));
        dirs.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));
        dirs
    }

    fn load_desktop_entries_from_dirs(dirs: &[PathBuf]) -> Vec<DesktopEntryMeta> {
        let mut entries = Vec::new();
        let mut stack = dirs.to_vec();
        while let Some(dir) = stack.pop() {
            if !dir.exists() {
                continue;
            }
            let read_dir = match std::fs::read_dir(&dir) {
                Ok(rd) => rd,
                Err(_) => continue,
            };
            for entry in read_dir.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().and_then(|s| s.to_str()) != Some("desktop") {
                    continue;
                }
                if let Some(meta) = Self::parse_desktop_entry(&path) {
                    entries.push(meta);
                }
            }
        }
        entries
    }

    fn parse_desktop_entry(path: &Path) -> Option<DesktopEntryMeta> {
        let content = std::fs::read_to_string(path).ok()?;
        let mut in_desktop_entry = false;
        let mut name = String::new();
        let mut icon = String::new();
        let mut exec = String::new();
        let mut startup_wm_class = String::new();
        for raw_line in content.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                in_desktop_entry = line == "[Desktop Entry]";
                continue;
            }
            if !in_desktop_entry {
                continue;
            }
            if let Some(value) = line.strip_prefix("Name=") {
                if name.is_empty() {
                    name = value.trim().to_string();
                }
                continue;
            }
            if let Some(value) = line.strip_prefix("Icon=") {
                if icon.is_empty() {
                    icon = value.trim().to_string();
                }
                continue;
            }
            if let Some(value) = line.strip_prefix("Exec=") {
                if exec.is_empty() {
                    exec = value.trim().to_string();
                }
                continue;
            }
            if let Some(value) = line.strip_prefix("StartupWMClass=") {
                if startup_wm_class.is_empty() {
                    startup_wm_class = value.trim().to_string();
                }
            }
        }
        if icon.is_empty() {
            return None;
        }
        let file_stem = path.file_stem()?.to_string_lossy().to_string();
        Some(DesktopEntryMeta {
            name,
            icon,
            exec,
            startup_wm_class,
            file_stem,
        })
    }

    fn load_desktop_entries_if_needed(&mut self) {
        if self.desktop_entries.is_none() {
            let dirs = Self::applications_dirs();
            self.desktop_entries = Some(Self::load_desktop_entries_from_dirs(&dirs));
        }
    }

    fn resolve_desktop_icon_with_entries(
        item: &AppletItem,
        entries: &[DesktopEntryMeta],
    ) -> Option<String> {
        let title = item.title.to_lowercase();
        let app_id = item.app_id.to_lowercase();
        let icon_name = item.icon_name.to_lowercase();
        let service = item.service_name.to_lowercase();
        let service_tail = service
            .rsplit('.')
            .next()
            .unwrap_or(service.as_str())
            .to_string();

        let mut best: Option<(i32, &DesktopEntryMeta)> = None;
        for entry in entries {
            let name = entry.name.to_lowercase();
            let exec = entry.exec.to_lowercase();
            let wm = entry.startup_wm_class.to_lowercase();
            let stem = entry.file_stem.to_lowercase();

            let mut score = 0;
            if !title.is_empty() && (name.contains(&title) || title.contains(&name)) {
                score += 120;
            }
            if !title.is_empty() && wm == title {
                score += 160;
            }
            if !service_tail.is_empty()
                && (stem.contains(&service_tail) || exec.contains(&service_tail))
            {
                score += 100;
            }
            if !service_tail.is_empty() && wm.contains(&service_tail) {
                score += 120;
            }
            if !icon_name.is_empty()
                && (entry.icon.to_lowercase() == icon_name || stem == icon_name)
            {
                score += 90;
            }
            if !app_id.is_empty()
                && (stem.contains(&app_id)
                    || exec.contains(&app_id)
                    || entry.icon.to_lowercase() == app_id
                    || wm.contains(&app_id))
            {
                score += 140;
            }

            if score > 0 {
                match best {
                    Some((best_score, _)) if best_score >= score => {}
                    _ => best = Some((score, entry)),
                }
            }
        }
        best.map(|(_, entry)| entry.icon.clone())
    }

    fn resolve_desktop_icon(&mut self, item: &AppletItem) -> Option<String> {
        let descriptor = Self::item_descriptor(item);
        if let Some(cached) = self.desktop_icon_cache.get(&descriptor) {
            return cached.clone();
        }
        self.load_desktop_entries_if_needed();
        let resolved = self
            .desktop_entries
            .as_ref()
            .and_then(|entries| Self::resolve_desktop_icon_with_entries(item, entries));
        self.desktop_icon_cache.insert(descriptor, resolved.clone());
        resolved
    }

    fn icon_candidates(&self, item: &AppletItem) -> Vec<String> {
        let mut candidates = Vec::new();
        let resolved = self
            .resolved_icon_by_item
            .get(&Self::item_id(item))
            .cloned()
            .unwrap_or_default();
        if !resolved.is_empty() {
            candidates.push(resolved);
        }
        if !item.icon_name.is_empty() && !candidates.iter().any(|c| c == &item.icon_name) {
            candidates.push(item.icon_name.clone());
        }
        candidates
    }

    fn refresh_resolved_icons(&mut self) {
        self.resolved_icon_by_item.clear();
        let items = self.items.clone();
        for item in &items {
            let key = Self::item_id(item);
            let resolved = self
                .resolve_desktop_icon(item)
                .or_else(|| (!item.icon_name.is_empty()).then(|| item.icon_name.clone()))
                .unwrap_or_default();
            self.resolved_icon_by_item.insert(key, resolved);
        }
    }

    fn ensure_icon_cached(&mut self, icon_key: &str, scale: f32) {
        if self.icon_cache.contains_key(icon_key) {
            return;
        }
        self.icon_cache
            .insert(icon_key.to_string(), self.load_icon_bitmap(icon_key, scale));
    }

    fn ensure_visible_icons_cached(&mut self, scale: f32) {
        if !self.show_icons {
            return;
        }
        for i in 0..self.visible_count() {
            for icon_key in self.icon_candidates(&self.items[i]) {
                self.ensure_icon_cached(&icon_key, scale);
            }
        }
    }

    fn draw_icon(
        &self,
        pixmap: &mut PixmapMut,
        context: &RenderContext,
        icon: &IconBitmap,
        x: f32,
        y: f32,
    ) {
        let scale = context.scale();
        let dst_w = ((self.icon_size as f32) * scale).max(1.0) as u32;
        let dst_h = ((self.icon_size as f32) * scale).max(1.0) as u32;
        let start_x = (x * scale).round() as i32;
        let start_y = (y * scale).round() as i32;

        let mut paint = Paint::default();
        let pixel = |x: u32, y: u32| -> [f32; 4] {
            let idx = ((y * icon.width + x) * 4) as usize;
            [
                icon.rgba_pixels[idx] as f32,
                icon.rgba_pixels[idx + 1] as f32,
                icon.rgba_pixels[idx + 2] as f32,
                icon.rgba_pixels[idx + 3] as f32,
            ]
        };
        for dy in 0..dst_h {
            for dx in 0..dst_w {
                let src_fx = ((dx as f32 + 0.5) * icon.width as f32 / dst_w as f32) - 0.5;
                let src_fy = ((dy as f32 + 0.5) * icon.height as f32 / dst_h as f32) - 0.5;

                let x0 = src_fx
                    .floor()
                    .clamp(0.0, icon.width.saturating_sub(1) as f32)
                    as u32;
                let y0 = src_fy
                    .floor()
                    .clamp(0.0, icon.height.saturating_sub(1) as f32)
                    as u32;
                let x1 = (x0 + 1).min(icon.width.saturating_sub(1));
                let y1 = (y0 + 1).min(icon.height.saturating_sub(1));

                let wx = (src_fx - x0 as f32).clamp(0.0, 1.0);
                let wy = (src_fy - y0 as f32).clamp(0.0, 1.0);

                let c00 = pixel(x0, y0);
                let c10 = pixel(x1, y0);
                let c01 = pixel(x0, y1);
                let c11 = pixel(x1, y1);

                let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
                let mix_row0 = [
                    lerp(c00[0], c10[0], wx),
                    lerp(c00[1], c10[1], wx),
                    lerp(c00[2], c10[2], wx),
                    lerp(c00[3], c10[3], wx),
                ];
                let mix_row1 = [
                    lerp(c01[0], c11[0], wx),
                    lerp(c01[1], c11[1], wx),
                    lerp(c01[2], c11[2], wx),
                    lerp(c01[3], c11[3], wx),
                ];

                let r = lerp(mix_row0[0], mix_row1[0], wy).round() as u8;
                let g = lerp(mix_row0[1], mix_row1[1], wy).round() as u8;
                let b = lerp(mix_row0[2], mix_row1[2], wy).round() as u8;
                let a = lerp(mix_row0[3], mix_row1[3], wy).round() as u8;
                if a == 0 {
                    continue;
                }

                let px = start_x + dx as i32;
                let py = start_y + dy as i32;
                if px < 0 || py < 0 || px as u32 >= pixmap.width() || py as u32 >= pixmap.height() {
                    continue;
                }

                paint.set_color(Color::from_rgba8(r, g, b, a));
                pixmap.fill_rect(
                    Rect::from_xywh(px as f32, py as f32, 1.0, 1.0).unwrap(),
                    &paint,
                    Transform::identity(),
                    None,
                );
            }
        }
    }
}

impl CrankyModule for AppletModule {
    type Error = AppletError;
    type Config = AppletConfig;

    fn init(
        &mut self,
        config: Self::Config,
        _bar_config: &crate::config::BarConfig,
    ) -> Result<(), Self::Error> {
        self.refresh_interval = Duration::from_millis(config.refresh_ms());
        self.show_titles = config.show_titles();
        self.show_icons = config.show_icons();
        self.icon_size = config.icon_size();
        self.icon_theme = config.icon_theme().map(str::to_string);
        self.max_items = config.max_items();
        self.empty_label = config.empty_label().to_string();
        Ok(())
    }

    fn update(&mut self, event: Event) -> UpdateAction {
        if !matches!(event, Event::Timer) {
            return UpdateAction::None;
        }

        let now = Instant::now();
        if !self.should_refresh(now) {
            return UpdateAction::None;
        }
        self.last_refresh = Some(now);

        match self.provider.list_items() {
            Ok(new_items) => {
                let mut redraw = false;
                let render_scale =
                    f32::from_bits(self.render_scale_bits.load(Ordering::Relaxed)).max(1.0);
                if (self.icon_cache_scale - render_scale).abs() > f32::EPSILON {
                    self.icon_cache.clear();
                    self.icon_cache_scale = render_scale;
                    redraw = true;
                }
                if self.items != new_items {
                    self.items = new_items;
                    self.refresh_resolved_icons();
                    redraw = true;
                }
                self.ensure_visible_icons_cached(render_scale);
                if self.error_message.take().is_some() {
                    redraw = true;
                }
                if redraw {
                    UpdateAction::Redraw
                } else {
                    UpdateAction::None
                }
            }
            Err(error) => {
                let message = error.to_string();
                if self.error_message.as_deref() == Some(message.as_str()) {
                    UpdateAction::None
                } else {
                    self.error_message = Some(message);
                    UpdateAction::Redraw
                }
            }
        }
    }

    fn view(
        &self,
        pixmap: &mut PixmapMut,
        area: Rect,
        context: &mut RenderContext,
        _monitor: &str,
    ) {
        self.render_scale_bits
            .store(context.scale().max(1.0).to_bits(), Ordering::Relaxed);
        let styling = self.text_styling();
        let y_offset = context.calculate_vertical_offset(area, styling.line_height());
        if let Some(label) = self.render_empty_or_error_label() {
            context.render_text(pixmap, &label, styling, area.left(), y_offset);
            return;
        }

        let icon_size = self.icon_size as f32;
        let icon_y = area.top() + (area.height() - icon_size) / 2.0;
        let mut x = area.left();

        for i in 0..self.visible_count() {
            if i > 0 {
                x += ITEM_SPACING;
            }
            let item = &self.items[i];

            if self.show_icons {
                let icon = self
                    .icon_candidates(item)
                    .into_iter()
                    .find_map(|icon_key| self.icon_cache.get(&icon_key).and_then(|v| v.as_ref()));
                if let Some(icon) = icon {
                    self.draw_icon(pixmap, context, icon, x, icon_y);
                } else if let Some(letter) = Self::item_fallback_letter(item) {
                    let icon_rect = Rect::from_xywh(x, icon_y, icon_size, icon_size).unwrap();
                    let fallback = TextStyling::new(
                        (self.icon_size as f32 * 0.7).max(10.0),
                        icon_size,
                        Color::from_rgba8(192, 202, 245, 255),
                        self.font_family.clone(),
                    );
                    let letter_width = context.measure_text(&letter, fallback.clone());
                    let letter_x = x + (icon_size - letter_width).max(0.0) / 2.0;
                    let letter_y =
                        context.calculate_vertical_offset(icon_rect, fallback.line_height());
                    context.render_text(pixmap, &letter, fallback, letter_x, letter_y);
                }
                x += icon_size;
            }

            if self.show_titles || !self.show_icons {
                if self.show_icons {
                    x += ICON_TEXT_GAP;
                }
                let title = self.item_title(item);
                context.render_text(pixmap, &title, styling.clone(), x, y_offset);
                x += context.measure_text(&title, styling.clone());
            }
        }

        if self.items.len() > self.visible_count() {
            let overflow = format!(" +{}", self.items.len() - self.visible_count());
            x += ITEM_SPACING;
            context.render_text(pixmap, &overflow, styling.clone(), x, y_offset);
        }
    }

    fn measure(&self, context: &mut RenderContext, _monitor: &str) -> f32 {
        let styling = self.text_styling();
        if let Some(label) = self.render_empty_or_error_label() {
            return context.measure_text(&label, styling);
        }

        let icon_size = self.icon_size as f32;
        let mut total = 0.0;
        for i in 0..self.visible_count() {
            if i > 0 {
                total += ITEM_SPACING;
            }
            let item = &self.items[i];

            if self.show_icons {
                total += icon_size;
            }
            if self.show_titles || !self.show_icons {
                if self.show_icons {
                    total += ICON_TEXT_GAP;
                }
                total += context.measure_text(&self.item_title(item), styling.clone());
            }
        }
        if self.items.len() > self.visible_count() {
            total += ITEM_SPACING
                + context.measure_text(
                    &format!(" +{}", self.items.len() - self.visible_count()),
                    styling,
                );
        }
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BarConfig;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct MockAppletProvider {
        responses: Arc<Mutex<Vec<AppletResult<Vec<AppletItem>>>>>,
    }

    impl MockAppletProvider {
        fn new(responses: Vec<AppletResult<Vec<AppletItem>>>) -> Self {
            Self {
                responses: Arc::new(Mutex::new(responses)),
            }
        }
    }

    impl AppletProvider for MockAppletProvider {
        fn list_items(&self) -> AppletResult<Vec<AppletItem>> {
            let mut guard = self.responses.lock().unwrap();
            if guard.is_empty() {
                return Ok(Vec::new());
            }
            guard.remove(0)
        }
    }

    fn applet_item(service_name: &str, title: &str, status: &str, icon_name: &str) -> AppletItem {
        AppletItem {
            service_name: service_name.to_string(),
            object_path: "/StatusNotifierItem".to_string(),
            title: title.to_string(),
            app_id: String::new(),
            status: status.to_string(),
            icon_name: icon_name.to_string(),
        }
    }

    fn module_with_responses(responses: Vec<AppletResult<Vec<AppletItem>>>) -> AppletModule {
        AppletModule::with_provider(Box::new(MockAppletProvider::new(responses)))
    }

    fn temp_path(ext: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "cranky-applet-test-{}-{}.{}",
            std::process::id(),
            nanos,
            ext
        ))
    }

    fn temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "cranky-applet-test-dir-{}-{}",
            std::process::id(),
            nanos
        ))
    }

    #[test]
    fn test_applet_config_deserialization() {
        let json = r##"{
            "refresh_ms": 250,
            "show_titles": false,
            "show_icons": true,
            "icon_size": 20,
            "icon_theme": "Papirus-Dark",
            "max_items": 3,
            "empty_label": "none"
        }"##;
        let config: AppletConfig = serde_json::from_str(json).unwrap();

        assert_eq!(config.refresh_ms(), 250);
        assert!(!config.show_titles());
        assert!(config.show_icons());
        assert_eq!(config.icon_size(), 20);
        assert_eq!(config.icon_theme(), Some("Papirus-Dark"));
        assert_eq!(config.max_items(), 3);
        assert_eq!(config.empty_label(), "none");
    }

    #[test]
    fn test_applet_update_redraw_on_change() {
        let mut module = module_with_responses(vec![
            Ok(vec![applet_item(
                "org.kde.StatusNotifierItem-1-1",
                "nm-applet",
                "Active",
                "network",
            )]),
            Ok(vec![
                applet_item(
                    "org.kde.StatusNotifierItem-1-1",
                    "nm-applet",
                    "Active",
                    "network",
                ),
                applet_item(
                    "org.kde.StatusNotifierItem-2-1",
                    "blueman",
                    "Active",
                    "bluetooth",
                ),
            ]),
        ]);
        module
            .init(
                AppletConfig {
                    refresh_ms: 0,
                    ..AppletConfig::default()
                },
                &BarConfig::default(),
            )
            .unwrap();

        assert_eq!(module.update(Event::Timer), UpdateAction::Redraw);
        assert_eq!(module.update(Event::Timer), UpdateAction::Redraw);
    }

    #[test]
    fn test_applet_update_no_redraw_on_same_data() {
        let items = vec![applet_item(
            "org.kde.StatusNotifierItem-1-1",
            "nm-applet",
            "Active",
            "network",
        )];
        let mut module = module_with_responses(vec![Ok(items.clone()), Ok(items)]);
        module
            .init(
                AppletConfig {
                    refresh_ms: 0,
                    ..AppletConfig::default()
                },
                &BarConfig::default(),
            )
            .unwrap();

        assert_eq!(module.update(Event::Timer), UpdateAction::Redraw);
        assert_eq!(module.update(Event::Timer), UpdateAction::None);
    }

    #[test]
    fn test_applet_update_error_redraws_once() {
        let mut module = module_with_responses(vec![
            Err(AppletError::Provider("dbus down".to_string())),
            Err(AppletError::Provider("dbus down".to_string())),
        ]);
        module
            .init(
                AppletConfig {
                    refresh_ms: 0,
                    ..AppletConfig::default()
                },
                &BarConfig::default(),
            )
            .unwrap();

        assert_eq!(module.update(Event::Timer), UpdateAction::Redraw);
        assert_eq!(module.update(Event::Timer), UpdateAction::None);
    }

    #[test]
    fn test_applet_measure_and_view() {
        let mut module = module_with_responses(vec![Ok(vec![applet_item(
            "org.kde.StatusNotifierItem-1-1",
            "nm-applet",
            "Active",
            "network-wireless",
        )])]);
        module
            .init(
                AppletConfig {
                    refresh_ms: 0,
                    ..AppletConfig::default()
                },
                &BarConfig::default(),
            )
            .unwrap();
        let _ = module.update(Event::Timer);

        let mut pixmap_data = vec![0; 220 * 30 * 4];
        let mut pixmap = PixmapMut::from_bytes(&mut pixmap_data, 220, 30).unwrap();
        let mut context = RenderContext::new();
        let area = Rect::from_xywh(0.0, 0.0, 220.0, 30.0).unwrap();

        module.view(&mut pixmap, area, &mut context, "eDP-1");
        let width = module.measure(&mut context, "eDP-1");
        assert!(width > 0.0);
    }

    #[test]
    fn test_applet_update_ignores_non_timer() {
        let mut module = module_with_responses(vec![]);
        module
            .init(AppletConfig::default(), &BarConfig::default())
            .unwrap();

        assert_eq!(
            module.update(Event::HyprlandUpdate {
                workspaces: Vec::new(),
                monitors: Vec::new(),
            }),
            UpdateAction::None
        );
    }

    #[test]
    fn test_resolve_desktop_icon_prioritized() {
        let item = applet_item(
            "org.kde.StatusNotifierItem-1234-1",
            "Discord",
            "Active",
            "bluetooth",
        );
        let entries = vec![
            DesktopEntryMeta {
                name: "Discord".to_string(),
                icon: "discord".to_string(),
                exec: "discord".to_string(),
                startup_wm_class: "discord".to_string(),
                file_stem: "discord".to_string(),
            },
            DesktopEntryMeta {
                name: "Blue Moon".to_string(),
                icon: "blueman".to_string(),
                exec: "blueman".to_string(),
                startup_wm_class: "Blueman".to_string(),
                file_stem: "blueman".to_string(),
            },
        ];

        let resolved = AppletModule::resolve_desktop_icon_with_entries(&item, &entries);
        assert_eq!(resolved.as_deref(), Some("discord"));
    }

    #[test]
    fn test_icon_lookup_sizes_prioritizes_scaled_then_fallbacks() {
        let sizes = AppletModule::icon_lookup_sizes(16, 1.25);
        assert_eq!(sizes[0], 20);
        assert_eq!(sizes[1], 16);
        assert!(sizes.contains(&24));
        assert!(sizes.contains(&32));
        assert!(sizes.contains(&48));
        let mut unique = sizes.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), sizes.len());
    }

    #[test]
    fn test_icon_candidates_fallback_to_item_icon_name() {
        let mut module = module_with_responses(vec![]);
        let item = applet_item(
            "org.kde.StatusNotifierItem-1-1",
            "blueman",
            "Active",
            "bluetooth",
        );
        module
            .resolved_icon_by_item
            .insert(AppletModule::item_id(&item), "blueman".to_string());

        let candidates = module.icon_candidates(&item);
        assert_eq!(
            candidates,
            vec!["blueman".to_string(), "bluetooth".to_string()]
        );
    }

    #[test]
    fn test_item_title_falls_back_to_service_when_path_is_numeric() {
        let item = AppletItem {
            service_name: "org.blueman.Applet".to_string(),
            object_path: "/StatusNotifierItem/42".to_string(),
            title: String::new(),
            app_id: String::new(),
            status: "NeedsAttention".to_string(),
            icon_name: String::new(),
        };
        let module = module_with_responses(vec![]);
        assert_eq!(
            module.item_title(&item),
            "org.blueman.Applet [NeedsAttention]"
        );
    }

    #[test]
    fn test_render_empty_or_error_label_priority() {
        let mut module = module_with_responses(vec![]);
        module.empty_label = "applet: none".to_string();
        module.error_message = Some("dbus down".to_string());
        assert_eq!(
            module.render_empty_or_error_label(),
            Some("applet: error (dbus down)".to_string())
        );

        module.error_message = None;
        assert_eq!(
            module.render_empty_or_error_label(),
            Some("applet: none".to_string())
        );

        module.items.push(applet_item(
            "org.kde.StatusNotifierItem-1-1",
            "nm-applet",
            "Active",
            "network",
        ));
        assert_eq!(module.render_empty_or_error_label(), None);
    }

    #[test]
    fn test_parse_desktop_entry_requires_desktop_entry_section() {
        let path = temp_path("desktop");
        fs::write(&path, "Name=Foo\nIcon=foo\n").unwrap();
        assert!(AppletModule::parse_desktop_entry(&path).is_none());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_load_icon_bitmap_from_absolute_png_path() {
        let path = temp_path("png");
        let mut rgba = image::RgbaImage::new(2, 2);
        rgba.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        rgba.save(&path).unwrap();

        let module = module_with_responses(vec![]);
        let bitmap = module.load_icon_bitmap(path.to_str().unwrap(), 1.0);
        let _ = fs::remove_file(&path);

        assert!(bitmap.is_some());
        let bitmap = bitmap.unwrap();
        assert_eq!(bitmap.width, 2);
        assert_eq!(bitmap.height, 2);
    }

    #[test]
    fn test_load_icon_bitmap_from_absolute_svg_path() {
        let path = temp_path("svg");
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="16" height="16" fill="#00ff00"/></svg>"##;
        fs::write(&path, svg).unwrap();

        let module = module_with_responses(vec![]);
        let bitmap = module.load_icon_bitmap(path.to_str().unwrap(), 1.0);
        let _ = fs::remove_file(&path);

        assert!(bitmap.is_some());
        let bitmap = bitmap.unwrap();
        assert!(bitmap.width >= 16);
        assert!(bitmap.height >= 16);
        assert!(bitmap.rgba_pixels.len() >= (bitmap.width * bitmap.height * 4) as usize);
    }

    #[test]
    fn test_item_fallback_letter_from_title() {
        let item = applet_item("org.kde.StatusNotifierItem-1-1", "discord", "Active", "");
        assert_eq!(
            AppletModule::item_fallback_letter(&item).as_deref(),
            Some("D")
        );
    }

    #[test]
    fn test_item_fallback_letter_from_service() {
        let item = applet_item("org.kde.StatusNotifierItem-1-1", "   ", "Active", "");
        assert_eq!(
            AppletModule::item_fallback_letter(&item).as_deref(),
            Some("O")
        );
    }

    #[test]
    fn test_visible_count_respects_max_and_minimum_one() {
        let mut module = module_with_responses(vec![]);
        module.items = vec![
            applet_item("s1", "a", "Active", "x"),
            applet_item("s2", "b", "Active", "y"),
            applet_item("s3", "c", "Active", "z"),
        ];
        module.max_items = 2;
        assert_eq!(module.visible_count(), 2);
        module.max_items = 0;
        assert_eq!(module.visible_count(), 1);
    }

    #[test]
    fn test_item_title_branches() {
        let mut module = module_with_responses(vec![]);
        let mut item = applet_item("org.test.App", "Title", "Active", "icon");
        item.app_id = "appid".to_string();
        assert_eq!(module.item_title(&item), "Title");

        item.title.clear();
        assert_eq!(module.item_title(&item), "appid");

        item.app_id.clear();
        assert_eq!(module.item_title(&item), "icon");

        item.icon_name.clear();
        item.object_path = "/org/test/MyTray".to_string();
        assert_eq!(module.item_title(&item), "MyTray");

        item.object_path = "/StatusNotifierItem/123".to_string();
        assert_eq!(module.item_title(&item), "org.test.App");

        item.status = "Passive".to_string();
        assert_eq!(module.item_title(&item), "org.test.App [Passive]");

        item.status.clear();
        assert_eq!(module.item_title(&item), "org.test.App");
        module.items.clear();
    }

    #[test]
    fn test_icon_candidates_empty_and_dedupe() {
        let mut module = module_with_responses(vec![]);
        let mut item = applet_item("org.kde.StatusNotifierItem-1-1", "a", "Active", "");
        assert!(module.icon_candidates(&item).is_empty());

        item.icon_name = "same".to_string();
        module
            .resolved_icon_by_item
            .insert(AppletModule::item_id(&item), "same".to_string());
        assert_eq!(module.icon_candidates(&item), vec!["same".to_string()]);
    }

    #[test]
    fn test_should_refresh_paths() {
        let mut module = module_with_responses(vec![]);
        module.refresh_interval = Duration::from_millis(10);
        let now = Instant::now();
        assert!(module.should_refresh(now));
        module.last_refresh = Some(now);
        assert!(!module.should_refresh(now + Duration::from_millis(5)));
        assert!(module.should_refresh(now + Duration::from_millis(11)));
    }

    #[test]
    fn test_parse_desktop_entry_first_values_and_section_scoping() {
        let path = temp_path("desktop");
        fs::write(
            &path,
            "[Other]\nIcon=ignored\n[Desktop Entry]\nName=One\nName=Two\nIcon=first\nIcon=second\nExec=cmd\nExec=cmd2\nStartupWMClass=wm1\nStartupWMClass=wm2\n",
        )
        .unwrap();

        let meta = AppletModule::parse_desktop_entry(&path).unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(meta.name, "One");
        assert_eq!(meta.icon, "first");
        assert_eq!(meta.exec, "cmd");
        assert_eq!(meta.startup_wm_class, "wm1");
    }

    #[test]
    fn test_load_desktop_entries_from_dirs_recursive_and_filtering() {
        let root = temp_dir();
        let nested = root.join("nested");
        fs::create_dir_all(&nested).unwrap();
        fs::write(root.join("ignore.txt"), "noop").unwrap();
        fs::write(
            root.join("top.desktop"),
            "[Desktop Entry]\nName=Top\nIcon=top\nExec=top\n",
        )
        .unwrap();
        fs::write(
            nested.join("inner.desktop"),
            "[Desktop Entry]\nName=Inner\nIcon=inner\nExec=inner\n",
        )
        .unwrap();

        let entries = AppletModule::load_desktop_entries_from_dirs(std::slice::from_ref(&root));
        let _ = fs::remove_dir_all(&root);

        let icons: Vec<_> = entries.into_iter().map(|e| e.icon).collect();
        assert!(icons.contains(&"top".to_string()));
        assert!(icons.contains(&"inner".to_string()));
    }

    #[test]
    fn test_refresh_resolved_icons_falls_back_to_item_icon_name() {
        let mut module = module_with_responses(vec![]);
        module.items = vec![applet_item("svc", "Title", "Active", "icon-from-item")];
        module.desktop_entries = Some(Vec::new());
        module.refresh_resolved_icons();
        let key = AppletModule::item_id(&module.items[0]);
        assert_eq!(
            module.resolved_icon_by_item.get(&key).cloned(),
            Some("icon-from-item".to_string())
        );
    }

    #[test]
    fn test_refresh_resolved_icons_uses_desktop_resolution() {
        let mut module = module_with_responses(vec![]);
        let item = applet_item("svc", "Discord", "Active", "fallback-icon");
        module.items = vec![item.clone()];
        module.desktop_entries = Some(vec![DesktopEntryMeta {
            name: "Discord".to_string(),
            icon: "discord".to_string(),
            exec: "discord".to_string(),
            startup_wm_class: "discord".to_string(),
            file_stem: "discord".to_string(),
        }]);
        module.refresh_resolved_icons();
        let key = AppletModule::item_id(&item);
        assert_eq!(
            module.resolved_icon_by_item.get(&key).cloned(),
            Some("discord".to_string())
        );
    }

    #[test]
    fn test_resolve_desktop_icon_uses_cache() {
        let mut module = module_with_responses(vec![]);
        let item = applet_item("svc", "App", "Active", "icon");
        let descriptor = AppletModule::item_descriptor(&item);
        module
            .desktop_icon_cache
            .insert(descriptor, Some("cached-icon".to_string()));
        assert_eq!(
            module.resolve_desktop_icon(&item),
            Some("cached-icon".to_string())
        );
    }

    #[test]
    fn test_ensure_visible_icons_cached_noop_when_icons_disabled() {
        let mut module = module_with_responses(vec![]);
        module.show_icons = false;
        module.items = vec![applet_item("svc", "App", "Active", "icon")];
        module.ensure_visible_icons_cached(1.0);
        assert!(module.icon_cache.is_empty());
    }

    #[test]
    fn test_item_descriptor_and_id() {
        let mut item = applet_item("svc", "Title", "Active", "icon");
        item.object_path = "/StatusNotifierItem/7".to_string();
        item.app_id = "appid".to_string();
        assert_eq!(
            AppletModule::item_descriptor(&item),
            "svc|Title|appid|icon".to_string()
        );
        assert_eq!(AppletModule::item_id(&item), "svc/StatusNotifierItem/7");
    }

    #[test]
    fn test_resolve_desktop_icon_with_entries_by_icon_name() {
        let item = applet_item("svc", "", "Active", "target-icon");
        let entries = vec![DesktopEntryMeta {
            name: "Whatever".to_string(),
            icon: "target-icon".to_string(),
            exec: "whatever".to_string(),
            startup_wm_class: String::new(),
            file_stem: "whatever".to_string(),
        }];
        assert_eq!(
            AppletModule::resolve_desktop_icon_with_entries(&item, &entries),
            Some("target-icon".to_string())
        );
    }

    #[test]
    fn test_resolve_desktop_icon_with_entries_by_app_id() {
        let mut item = applet_item("svc", "", "Active", "");
        item.app_id = "discord".to_string();
        let entries = vec![DesktopEntryMeta {
            name: "Discord".to_string(),
            icon: "discord".to_string(),
            exec: "discord --start".to_string(),
            startup_wm_class: "discord".to_string(),
            file_stem: "discord".to_string(),
        }];
        assert_eq!(
            AppletModule::resolve_desktop_icon_with_entries(&item, &entries),
            Some("discord".to_string())
        );
    }

    #[test]
    fn test_update_redraws_when_icon_cache_scale_changes() {
        let mut module = module_with_responses(vec![Ok(Vec::new())]);
        module
            .init(
                AppletConfig {
                    refresh_ms: 0,
                    ..AppletConfig::default()
                },
                &BarConfig::default(),
            )
            .unwrap();
        module.icon_cache.insert("x".to_string(), None);
        module.icon_cache_scale = 2.0;
        module.render_scale_bits.store(1.0f32.to_bits(), Ordering::Relaxed);

        assert_eq!(module.update(Event::Timer), UpdateAction::Redraw);
        assert_eq!(module.icon_cache_scale, 1.0);
        assert!(module.icon_cache.is_empty());
    }

    #[test]
    fn test_resolve_desktop_icon_with_entries_none_when_no_match() {
        let item = applet_item("svc", "Unknown", "Active", "icon-x");
        let entries = vec![DesktopEntryMeta {
            name: "Other".to_string(),
            icon: "other".to_string(),
            exec: "other".to_string(),
            startup_wm_class: "other".to_string(),
            file_stem: "other".to_string(),
        }];
        assert!(AppletModule::resolve_desktop_icon_with_entries(&item, &entries).is_none());
    }

    #[test]
    fn test_applications_dirs_contains_system_path() {
        let dirs = AppletModule::applications_dirs();
        assert!(dirs.contains(&PathBuf::from("/usr/share/applications")));
    }

    #[test]
    fn test_load_desktop_entries_if_needed_respects_existing_cache() {
        let mut module = module_with_responses(vec![]);
        module.desktop_entries = Some(vec![DesktopEntryMeta {
            name: "Cached".to_string(),
            icon: "cached".to_string(),
            exec: "cached".to_string(),
            startup_wm_class: "cached".to_string(),
            file_stem: "cached".to_string(),
        }]);
        module.load_desktop_entries_if_needed();
        assert_eq!(module.desktop_entries.as_ref().map(|e| e.len()), Some(1));
        assert_eq!(
            module.desktop_entries.as_ref().unwrap()[0].icon,
            "cached".to_string()
        );
    }

    #[test]
    fn test_measure_includes_overflow_counter() {
        let mut module = module_with_responses(vec![]);
        module.show_icons = true;
        module.show_titles = true;
        module.max_items = 1;
        module.items = vec![
            applet_item("svc1", "One", "Active", "icon1"),
            applet_item("svc2", "Two", "Active", "icon2"),
            applet_item("svc3", "Three", "Active", "icon3"),
        ];
        let mut context = RenderContext::new();
        let width = module.measure(&mut context, "eDP-1");
        assert!(width > module.icon_size as f32);
    }

    #[test]
    fn test_view_title_only_without_icons() {
        let mut module = module_with_responses(vec![]);
        module.show_icons = false;
        module.show_titles = true;
        module.items = vec![
            applet_item("svc1", "One", "Active", "icon1"),
            applet_item("svc2", "Two", "Active", "icon2"),
        ];

        let mut pixmap_data = vec![0; 260 * 30 * 4];
        let mut pixmap = PixmapMut::from_bytes(&mut pixmap_data, 260, 30).unwrap();
        let mut context = RenderContext::new();
        let area = Rect::from_xywh(0.0, 0.0, 260.0, 30.0).unwrap();
        module.view(&mut pixmap, area, &mut context, "eDP-1");
        assert!(module.measure(&mut context, "eDP-1") > 0.0);
    }

    #[test]
    fn test_update_skips_when_refresh_interval_not_elapsed() {
        let mut module = module_with_responses(vec![Ok(Vec::new())]);
        module
            .init(
                AppletConfig {
                    refresh_ms: 60_000,
                    ..AppletConfig::default()
                },
                &BarConfig::default(),
            )
            .unwrap();
        module.last_refresh = Some(Instant::now());
        assert_eq!(module.update(Event::Timer), UpdateAction::None);
    }

    #[test]
    fn test_update_redraws_when_error_message_clears() {
        let initial = vec![applet_item("svc1", "One", "Active", "icon1")];
        let mut module = module_with_responses(vec![Ok(initial.clone())]);
        module
            .init(
                AppletConfig {
                    refresh_ms: 0,
                    ..AppletConfig::default()
                },
                &BarConfig::default(),
            )
            .unwrap();
        module.items = initial;
        module.error_message = Some("previous error".to_string());
        assert_eq!(module.update(Event::Timer), UpdateAction::Redraw);
        assert!(module.error_message.is_none());
    }

    #[test]
    fn test_ensure_visible_icons_cached_limits_to_visible_items() {
        let icon_path = temp_path("png");
        let mut rgba = image::RgbaImage::new(2, 2);
        rgba.put_pixel(0, 0, image::Rgba([1, 2, 3, 255]));
        rgba.save(&icon_path).unwrap();
        let icon_string = icon_path.to_string_lossy().to_string();

        let mut module = module_with_responses(vec![]);
        module.show_icons = true;
        module.max_items = 1;
        module.items = vec![
            applet_item("svc1", "One", "Active", &icon_string),
            applet_item("svc2", "Two", "Active", "non-existent-icon"),
        ];
        module.refresh_resolved_icons();
        module.ensure_visible_icons_cached(1.0);
        let _ = fs::remove_file(&icon_path);

        assert_eq!(module.icon_cache.len(), 1);
    }

    #[test]
    fn test_parse_registered_item_service_and_path() {
        let endpoint =
            RealAppletProvider::parse_registered_item(":1.42/org/ayatana/NotificationItem/discord");
        assert_eq!(
            endpoint,
            Some((
                ":1.42".to_string(),
                "/org/ayatana/NotificationItem/discord".to_string()
            ))
        );
    }

    #[test]
    fn test_parse_registered_item_service_only() {
        let endpoint =
            RealAppletProvider::parse_registered_item("org.kde.StatusNotifierItem-1234-1");
        assert_eq!(
            endpoint,
            Some((
                "org.kde.StatusNotifierItem-1234-1".to_string(),
                "/StatusNotifierItem".to_string()
            ))
        );
    }

    #[test]
    fn test_normalize_registered_item_path_only_uses_sender() {
        let endpoint =
            normalize_registered_item("/org/ayatana/NotificationItem/discord", Some(":1.523"));
        assert_eq!(
            endpoint,
            Some((
                ":1.523".to_string(),
                "/org/ayatana/NotificationItem/discord".to_string()
            ))
        );
    }

    #[test]
    fn test_normalize_registered_item_path_only_without_sender() {
        let endpoint = normalize_registered_item("/org/ayatana/NotificationItem/discord", None);
        assert_eq!(endpoint, None);
    }
}
