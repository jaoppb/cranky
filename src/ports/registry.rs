use crate::domain::commands::AppCommand;
use crate::domain::config::{BarConfig, Config, ModuleConfig};
use crate::domain::events::PointerEvent;
use crate::domain::signals::{SignalHub, SignalKind};
use crate::domain::{
    ModuleId, MonitorId,
    shared::geometry::{Rect, Size},
};
use crate::ports::canvas::Canvas;
use crate::ports::surface::DynSurfaceManager;
use async_trait::async_trait;
use std::sync::Arc;
pub trait CommandSender: Send + Sync {
    fn send_command(&self, cmd: AppCommand);
}

pub trait LayoutSender: Send + Sync {
    fn send_layout(&self, layout: std::collections::HashMap<MonitorId, Rect>);
    fn current_layout(&self) -> std::collections::HashMap<MonitorId, Rect>;
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
        command_tx: Arc<dyn CommandSender>,
    ) -> std::collections::HashMap<ModuleId, Box<dyn LayoutSender>>;

    fn left_modules(&self) -> Vec<ModuleId>;
    fn center_modules(&self) -> Vec<ModuleId>;
    fn right_modules(&self) -> Vec<ModuleId>;

    fn clear(&mut self);

    async fn register_dbus_subscriptions(&self, dbus: &mut dyn crate::ports::DBusPort);
}
