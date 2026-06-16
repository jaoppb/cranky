use crate::domain::config::{Config, ModuleConfig, BarConfig};
use crate::domain::signals::{SignalHub, SignalKind};
use crate::domain::{ModuleId, MonitorId, geometry::{Size, Rect}};
use crate::ports::surface::DynSurfaceManager;
use crate::domain::commands::AppCommand;
use crate::domain::events::InputEvent;
use crate::ports::canvas::Canvas;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{watch, mpsc};

pub struct ModuleContext {
    pub id: ModuleId,
    pub hub: Arc<SignalHub>,
    pub surface_manager: DynSurfaceManager,
    pub command_tx: mpsc::Sender<AppCommand>,
    // The registry/app will send layout bounds for each monitor
    pub layout_rx: watch::Receiver<std::collections::HashMap<MonitorId, Rect>>,
}

#[async_trait]
pub trait AnyModulePort: Send + Sync {
    fn init(&mut self, config: &ModuleConfig, bar_config: &BarConfig) -> Result<(), String>;
    fn subscriptions(&self) -> Vec<SignalKind>;
    fn refresh(&mut self, hub: &SignalHub);
    fn view(&self, canvas: &mut dyn Canvas, monitor: &MonitorId);
    fn measure(&self, canvas: &mut dyn Canvas, monitor: &MonitorId) -> Size;
    fn on_event(&mut self, event: InputEvent) -> Vec<AppCommand>;
}

#[async_trait]
pub trait ModuleRegistryPort: Send + Sync {
    fn load(&mut self, config: &Config) -> Result<(), String>;
    fn spawn_all(
        &mut self,
        hub: Arc<SignalHub>,
        surface_manager: DynSurfaceManager,
        command_tx: mpsc::Sender<AppCommand>
    ) -> std::collections::HashMap<ModuleId, watch::Sender<std::collections::HashMap<MonitorId, Rect>>>;
    
    fn left_modules(&self) -> Vec<ModuleId>;
    fn center_modules(&self) -> Vec<ModuleId>;
    fn right_modules(&self) -> Vec<ModuleId>;
    
    fn clear(&mut self);
    
    async fn register_dbus_subscriptions(&self, dbus: &mut dyn crate::ports::DBusPort);
}
