use crate::domain::config::{Config, ModuleConfig, BarConfig};
use crate::domain::signals::{SignalHub, SignalKind};
use crate::domain::{ModuleId, MonitorId, geometry::Size};
use crate::ports::canvas::Canvas;
use crate::domain::events::InputEvent;
use crate::domain::commands::AppCommand;
use async_trait::async_trait;

pub trait AnyModulePort: Send + Sync {
    fn init(
        &mut self,
        config: &ModuleConfig,
        bar_config: &BarConfig,
    ) -> Result<(), String>;

    fn subscriptions(&self) -> Vec<SignalKind>;

    fn refresh(&mut self, hub: &SignalHub);

    fn view(&self, canvas: &mut dyn Canvas, monitor: &MonitorId);

    fn measure(&self, canvas: &mut dyn Canvas, monitor: &MonitorId) -> Size;

    fn on_event(&mut self, event: InputEvent) -> Vec<AppCommand>;
}

#[async_trait]
pub trait ModuleRegistryPort: Send + Sync {
    fn load(&mut self, config: &Config) -> Result<(), String>;
    fn attach_all(&mut self, hub: &SignalHub);
    fn refresh_all(&mut self, hub: &SignalHub);
    fn left_modules(&self) -> Vec<ModuleId>;
    fn center_modules(&self) -> Vec<ModuleId>;
    fn right_modules(&self) -> Vec<ModuleId>;
    fn get(&self, id: ModuleId) -> Option<&dyn AnyModulePort>;
    fn get_mut(&mut self, id: ModuleId) -> Option<&mut dyn AnyModulePort>;
    async fn register_dbus_subscriptions(&self, dbus: &mut dyn crate::ports::DBusPort);
}
