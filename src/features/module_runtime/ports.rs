use crate::app::commands::AppCommand;
use crate::shared::config::domain::{Config, ModuleConfig};
use crate::shared::events::signals::{SignalHub, SignalKind};
use crate::shared::primitives::{
    ModuleId, MonitorId,
    geometry::Rect,
};
use crate::shared::wayland::ports::DynSurfaceManager;
use async_trait::async_trait;
use std::sync::Arc;
pub trait CommandSender: Send + Sync {
    fn send_command(&self, cmd: AppCommand);
}

pub trait LayoutSender: Send + Sync {
    fn send_layout(&self, layout: std::collections::HashMap<MonitorId, Rect>);
}
#[async_trait]
pub trait AnyModulePort: Send + Sync {
    fn init(&mut self, config: &ModuleConfig, full_config: &Config) -> Result<(), String>;
    fn subscriptions(&self) -> Vec<SignalKind>;
    fn refresh(&mut self, hub: &SignalHub, changed_signals: &[SignalKind]);
    fn render(&self, monitor: &MonitorId) -> crate::features::layout_engine::domain::LayoutNode;
}

#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait ModuleRegistryPort<Fact: crate::shared::rendering::ports::canvas::CanvasFactory + 'static>: Send + Sync {
    fn load(&mut self, config: &Config) -> Result<(), String>;
    fn spawn_all(
        &mut self,
        hub: Arc<SignalHub>,
        surface_manager: DynSurfaceManager,
        command_tx: Arc<dyn CommandSender>,
        canvas_factory: Arc<std::sync::Mutex<Fact>>,
    ) -> std::collections::HashMap<ModuleId, Box<dyn LayoutSender>>;

    fn left_modules(&self) -> Vec<ModuleId>;
    fn center_modules(&self) -> Vec<ModuleId>;
    fn right_modules(&self) -> Vec<ModuleId>;

    fn clear(&mut self);

    async fn register_dbus_subscriptions(&self, dbus: &mut dyn crate::shared::dbus::ports::DBusPort);
}
