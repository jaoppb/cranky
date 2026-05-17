use crate::modules::CrankyModule;
use crate::ports::canvas::{Canvas};
use crate::domain::signals::{SignalHub, PointerEvent};
use crate::domain::errors::DomainError;
use crate::domain::color::DrawingColor;
use crate::domain::{ModuleId, MonitorId, geometry::Size};
use tracing::{debug};
use serde::Deserialize;
use std::collections::{HashMap};
use std::time::{Duration, Instant};
use thiserror::Error;

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

fn default_refresh_ms() -> u64 { 1000 }
fn default_show_titles() -> bool { true }
fn default_show_icons() -> bool { true }
fn default_icon_size() -> u16 { 16 }
fn default_max_items() -> usize { 6 }
fn default_empty_label() -> String { "applet: none".to_string() }

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

trait AppletProvider: Send + Sync {
    fn list_items(&self) -> AppletResult<Vec<AppletItem>>;
}

pub struct AppletModule {
    provider: Box<dyn AppletProvider>,
    items: Vec<AppletItem>,
    icon_cache: HashMap<String, Option<IconBitmap>>,
    resolved_icon_by_item: HashMap<String, String>,
    last_refresh: Option<Instant>,
    refresh_interval: Duration,
    show_titles: bool,
    show_icons: bool,
    icon_size: u16,
    icon_theme: Option<String>,
    max_items: usize,
    empty_label: String,
    error_message: Option<String>,
    target_id: ModuleId,
}

impl AppletModule {
    pub fn new() -> Self {
        struct DummyProvider;
        impl AppletProvider for DummyProvider {
            fn list_items(&self) -> AppletResult<Vec<AppletItem>> { Ok(Vec::new()) }
        }
        
        Self {
            provider: Box::new(DummyProvider),
            items: Vec::new(),
            icon_cache: HashMap::new(),
            resolved_icon_by_item: HashMap::new(),
            last_refresh: None,
            refresh_interval: Duration::from_millis(default_refresh_ms()),
            show_titles: default_show_titles(),
            show_icons: default_show_icons(),
            icon_size: default_icon_size(),
            icon_theme: None,
            max_items: default_max_items(),
            empty_label: default_empty_label(),
            error_message: None,
            target_id: ModuleId::new(0),
        }
    }

    fn item_title(&self, item: &AppletItem) -> String {
        let base = if !item.title.is_empty() {
            item.title.clone()
        } else if !item.app_id.is_empty() {
            item.app_id.clone()
        } else {
            item.service_name.clone()
        };

        if item.status == "Active" || item.status.is_empty() {
            base
        } else {
            format!("{} [{}]", base, item.status)
        }
    }
}

impl CrankyModule for AppletModule {
    type Config = AppletConfig;

    fn init(
        &mut self,
        config: Self::Config,
        _bar_config: &crate::config::BarConfig,
    ) -> Result<(), DomainError> {
        self.refresh_interval = Duration::from_millis(config.refresh_ms);
        self.show_titles = config.show_titles;
        self.show_icons = config.show_icons;
        self.icon_size = config.icon_size;
        self.icon_theme = config.icon_theme;
        self.max_items = config.max_items;
        self.empty_label = config.empty_label;
        Ok(())
    }

    fn attach(&mut self, hub: &SignalHub, target_id: ModuleId) {
        self.target_id = target_id;
        let mut time_rx = hub.time_rx();
        let dirty_tx = hub.dirty_tx();
        let mut pointer_rx = hub.subscribe_pointer();
        
        tokio::spawn(async move {
            while time_rx.changed().await.is_ok() {
                let _ = dirty_tx.send(target_id).await;
            }
        });

        tokio::spawn(async move {
            while let Ok(event) = pointer_rx.recv().await {
                match event {
                    PointerEvent::Click { target_id: tid, .. } if tid == target_id => {
                        debug!("Applet module clicked!");
                    }
                    _ => {}
                }
            }
        });
    }

    fn refresh(&mut self, _hub: &SignalHub) {
        let now = Instant::now();
        if self.last_refresh.map_or(true, |last| now.duration_since(last) >= self.refresh_interval) {
            self.last_refresh = Some(now);
            match self.provider.list_items() {
                Ok(items) => {
                    self.items = items;
                    self.error_message = None;
                }
                Err(e) => {
                    self.error_message = Some(e.to_string());
                }
            }
        }
    }

    fn view(&self, canvas: &mut dyn Canvas, _monitor: &MonitorId) {
        let text_color = DrawingColor::parse("#c0caf5").unwrap();
        let mut x = 0.0;
        
        if let Some(err) = &self.error_message {
            canvas.draw_text(&format!("error: {}", err), "", 14.0, text_color, 0.0, 0.0);
            return;
        }

        if self.items.is_empty() {
            canvas.draw_text(&self.empty_label, "", 14.0, text_color, 0.0, 0.0);
            return;
        }

        for (i, item) in self.items.iter().take(self.max_items).enumerate() {
            if i > 0 { x += ITEM_SPACING; }
            
            if self.show_icons {
                canvas.draw_rect(x, 0.0, 16.0, 16.0, text_color.clone(), 2.0);
                x += 16.0 + ICON_TEXT_GAP;
            }

            if self.show_titles {
                let title = self.item_title(item);
                let (w, h) = canvas.measure_text(&title, "", 14.0);
                canvas.draw_text(&title, "", 14.0, text_color.clone(), x, 0.0);
                x += w;
            }
        }
    }

    fn measure(&self, canvas: &mut dyn Canvas, _monitor: &MonitorId) -> Size {
        let mut total_w = 0.0;
        if self.items.is_empty() {
            let (w, h) = canvas.measure_text(&self.empty_label, "", 14.0);
            return Size::new(w.ceil() as u32, h.ceil() as u32);
        }

        for (i, item) in self.items.iter().take(self.max_items).enumerate() {
            if i > 0 { total_w += ITEM_SPACING; }
            if self.show_icons { total_w += 16.0 + ICON_TEXT_GAP; }
            if self.show_titles {
                let (w, _) = canvas.measure_text(&self.item_title(item), "", 14.0);
                total_w += w;
            }
        }
        Size::new(total_w.ceil() as u32, 30)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::canvas::MockCanvas;
    use crate::config::Config;
    use crate::domain::geometry::Point64;

    #[test]
    fn test_applet_module_init() {
        let mut module = AppletModule::new();
        let config = AppletConfig {
            refresh_ms: 500,
            show_titles: false,
            ..Default::default()
        };
        CrankyModule::init(&mut module, config, &crate::config::BarConfig::default()).unwrap();
        assert_eq!(module.refresh_interval, Duration::from_millis(500));
        assert!(!module.show_titles);
    }

    #[test]
    fn test_applet_module_view_empty() {
        let mut module = AppletModule::new();
        module.empty_label = "none".to_string();
        
        let mut mock = MockCanvas::new();
        mock.expect_draw_text()
            .withf(|text, _, _, _, _, _| text == "none")
            .times(1)
            .returning(|_, _, _, _, _, _| ());

        CrankyModule::view(&module, &mut mock, &MonitorId::new("eDP-1"));
    }

    #[tokio::test]
    async fn test_applet_module_reactive_pointer() {
        let (hub, _dirty_rx) = SignalHub::new(Config::default());
        let mut module = AppletModule::new();
        
        CrankyModule::attach(&mut module, &hub, ModuleId::new(7));
        assert_eq!(module.target_id, ModuleId::new(7));

        hub.pointer_tx().send(PointerEvent::Click { target_id: ModuleId::new(99), pos: Point64::new(0.0, 0.0), button: 0 }).unwrap();
        hub.pointer_tx().send(PointerEvent::Click { target_id: ModuleId::new(7), pos: Point64::new(10.0, 10.0), button: 272 }).unwrap();

        tokio::task::yield_now().await;
    }
}
