use crate::domain::config::{Config, ModuleConfig, BarConfig};
use crate::domain::signals::{SignalHub, SignalKind};
use crate::domain::{ModuleId, MonitorId, shared::geometry::{Size, Rect}};
use crate::ports::surface::DynSurfaceManager;
use crate::domain::commands::AppCommand;
use crate::domain::events::PointerEvent;
use crate::ports::canvas::Canvas;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{watch, mpsc};

pub struct ModuleContext {
    id: ModuleId,
    hub: Arc<SignalHub>,
    surface_manager: DynSurfaceManager,
    command_tx: mpsc::Sender<AppCommand>,
    // The registry/app will send layout bounds for each monitor
    layout_rx: watch::Receiver<std::collections::HashMap<MonitorId, Rect>>,
    pointer_rx: tokio::sync::broadcast::Receiver<(ModuleId, PointerEvent)>,
}

impl ModuleContext {
    pub fn new(
        id: ModuleId,
        hub: Arc<SignalHub>,
        surface_manager: DynSurfaceManager,
        command_tx: mpsc::Sender<AppCommand>,
        layout_rx: watch::Receiver<std::collections::HashMap<MonitorId, Rect>>,
    ) -> Self {
        let pointer_rx = hub.pointer_rx();
        Self {
            id,
            hub,
            surface_manager,
            command_tx,
            layout_rx,
            pointer_rx,
        }
    }

    pub fn id(&self) -> ModuleId {
        self.id
    }

    pub fn hub(&self) -> &Arc<SignalHub> {
        &self.hub
    }

    pub fn surface_manager(&self) -> &DynSurfaceManager {
        &self.surface_manager
    }

    pub fn command_tx(&self) -> &mpsc::Sender<AppCommand> {
        &self.command_tx
    }

    pub fn rxs_mut(&mut self) -> (
        &mut watch::Receiver<std::collections::HashMap<MonitorId, Rect>>,
        &mut tokio::sync::broadcast::Receiver<(ModuleId, PointerEvent)>
    ) {
        (&mut self.layout_rx, &mut self.pointer_rx)
    }
}

#[async_trait]
pub trait AnyModulePort: Send + Sync {
    fn init(&mut self, config: &ModuleConfig, bar_config: &BarConfig) -> Result<(), String>;
    fn subscriptions(&self) -> Vec<SignalKind>;
    fn refresh(&mut self, hub: &SignalHub);
    fn view(&self, canvas: &mut dyn Canvas, monitor: &MonitorId);
    fn measure(&self, canvas: &mut dyn Canvas, monitor: &MonitorId) -> Size;
    fn on_pointer_event(&mut self, event: PointerEvent) -> Vec<AppCommand>;
}

#[async_trait]
#[cfg_attr(test, mockall::automock)]
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
