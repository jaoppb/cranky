use crate::config::ModuleConfig;
use crate::ports::canvas::Canvas;
use crate::domain::signals::SignalHub;
use crate::domain::errors::DomainError;
use std::error::Error;

pub mod applet;
pub mod hour;
pub mod workspace;

/// A type-erased version of CrankyModule that is specialized for a specific Canvas implementation.
/// This allows for static dispatch performance while maintaining a heterogeneous registry.
pub trait AnyModule<C: Canvas>: Send + Sync {
    fn init(
        &mut self,
        config: &ModuleConfig,
        bar_config: &crate::config::BarConfig,
    ) -> Result<(), DomainError>;

    /// Attach the module to the signal hub and assign it a target ID for events.
    fn attach(&mut self, hub: &SignalHub, target_id: u32);

    /// Refresh: Synchronizes internal state from the signal hub.
    fn refresh(&mut self, hub: &SignalHub);

    /// View: Renders the current state of the module.
    fn view(&self, canvas: &mut C, monitor: &str);

    /// Measure: Returns the logical dimensions of the module.
    fn measure(&self, canvas: &mut C, monitor: &str) -> (f32, f32);
}

pub trait CrankyModule<C: Canvas>: Send + Sync {
    type Config: for<'de> serde::Deserialize<'de> + Default + Send + Sync;

    fn init(
        &mut self,
        config: Self::Config,
        bar_config: &crate::config::BarConfig,
    ) -> Result<(), DomainError>;

    fn attach(&mut self, hub: &SignalHub, target_id: u32);

    fn refresh(&mut self, hub: &SignalHub);

    fn view(&self, canvas: &mut C, monitor: &str);

    fn measure(&self, canvas: &mut C, monitor: &str) -> (f32, f32);
}

impl<T, C> AnyModule<C> for T
where
    T: CrankyModule<C>,
    C: Canvas + 'static,
{
    fn init(
        &mut self,
        config: &ModuleConfig,
        bar_config: &crate::config::BarConfig,
    ) -> Result<(), DomainError> {
        let json_value = serde_json::to_value(config.options())
            .map_err(|e| DomainError::ConfigParseError { reason: e.to_string() })?;

        let typed_config: T::Config = serde_json::from_value(json_value)
            .map_err(|e| DomainError::ConfigParseError { reason: e.to_string() })?;

        self.init(typed_config, bar_config)
    }

    fn attach(&mut self, hub: &SignalHub, target_id: u32) {
        self.attach(hub, target_id);
    }

    fn refresh(&mut self, hub: &SignalHub) {
        self.refresh(hub);
    }

    fn view(&self, canvas: &mut C, monitor: &str) {
        self.view(canvas, monitor);
    }

    fn measure(&self, canvas: &mut C, monitor: &str) -> (f32, f32) {
        self.measure(canvas, monitor)
    }
}

pub struct ModuleRegistry<C: Canvas> {
    left_modules: Vec<Box<dyn AnyModule<C>>>,
    center_modules: Vec<Box<dyn AnyModule<C>>>,
    right_modules: Vec<Box<dyn AnyModule<C>>>,
}

impl<C: Canvas + 'static> ModuleRegistry<C> {
    pub fn new() -> Self {
        Self {
            left_modules: Vec::new(),
            center_modules: Vec::new(),
            right_modules: Vec::new(),
        }
    }

    pub fn left_modules(&self) -> &[Box<dyn AnyModule<C>>] {
        &self.left_modules
    }

    pub fn center_modules(&self) -> &[Box<dyn AnyModule<C>>] {
        &self.center_modules
    }

    pub fn right_modules(&self) -> &[Box<dyn AnyModule<C>>] {
        &self.right_modules
    }

    pub fn load(&mut self, config: &crate::config::Config) -> Result<(), DomainError> {
        self.left_modules = self.create_modules(config.modules().left(), config.bar())?;
        self.center_modules = self.create_modules(config.modules().center(), config.bar())?;
        self.right_modules = self.create_modules(config.modules().right(), config.bar())?;
        Ok(())
    }

    fn create_modules(
        &self,
        configs: &[ModuleConfig],
        bar_config: &crate::config::BarConfig,
    ) -> Result<Vec<Box<dyn AnyModule<C>>>, DomainError> {
        let mut modules = Vec::new();
        for config in configs {
            if !config.is_enabled() {
                continue;
            }

            let mut module: Box<dyn AnyModule<C>> = match config.name() {
                "hour" => Box::new(hour::HourModule::new()),
                "applet" => Box::new(applet::AppletModule::new()),
                "workspace" => Box::new(workspace::WorkspaceModule::new()),
                name => return Err(DomainError::ModuleNotFound { module_name: name.to_string() }),
            };

            module.init(config, bar_config)?;
            modules.push(module);
        }
        Ok(modules)
    }
}
