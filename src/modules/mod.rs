use crate::config::ModuleConfig;
use crate::render::RenderContext;
use std::error::Error;
use thiserror::Error;
use tiny_skia::PixmapMut;

pub mod applet;
pub mod hour;
pub mod workspace;

#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("Module not found: {0}")]
    NotFound(String),
    #[error("Module initialization failed for '{0}': {1}")]
    InitFailed(String, Box<dyn Error + Send + Sync>),
}

pub type RegistryResult<T> = std::result::Result<T, RegistryError>;

#[derive(Debug, Clone)]
pub enum Event {
    Timer,
    HyprlandUpdate {
        workspaces: Vec<crate::core::hyprland::Workspace>,
        monitors: Vec<crate::core::hyprland::Monitor>,
    },
    PointerEnter,
    PointerLeave,
    Click {
        x: f64,
        y: f64,
        button: u32,
    },
    Scroll {
        axis: u32,
        value: f64,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateAction {
    None,
    Redraw,
}

/// A type-erased version of CrankyModule following Elm architecture principles.
pub trait AnyModule: Send + Sync {
    fn init(
        &mut self,
        config: &ModuleConfig,
        bar_config: &crate::config::BarConfig,
    ) -> std::result::Result<(), Box<dyn Error + Send + Sync>>;

    /// Update: Mutates the state (Model) based on an Event (Msg).
    fn update(&mut self, event: Event) -> UpdateAction;

    /// View: A pure function that renders the current state (Model).
    fn view(&self, pixmap: &mut PixmapMut, context: &mut RenderContext, monitor: &str);

    /// Measure: Returns the width of the module.
    fn measure(&self, context: &mut RenderContext, monitor: &str) -> f32;
}

pub trait CrankyModule: Send + Sync {
    type Error: Error + Send + Sync + 'static;
    type Config: for<'de> serde::Deserialize<'de> + Default + Send + Sync;

    fn init(
        &mut self,
        config: Self::Config,
        bar_config: &crate::config::BarConfig,
    ) -> std::result::Result<(), Self::Error>;
    fn update(&mut self, event: Event) -> UpdateAction;
    fn view(&self, pixmap: &mut PixmapMut, context: &mut RenderContext, monitor: &str);
    fn measure(&self, context: &mut RenderContext, monitor: &str) -> f32;
}

impl<T, E, C> AnyModule for T
where
    T: CrankyModule<Error = E, Config = C>,
    E: Error + Send + Sync + 'static,
    C: for<'de> serde::Deserialize<'de> + Default + Send + Sync,
{
    fn init(
        &mut self,
        config: &ModuleConfig,
        bar_config: &crate::config::BarConfig,
    ) -> std::result::Result<(), Box<dyn Error + Send + Sync>> {
        let json_value = serde_json::to_value(config.options())
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

        let typed_config: C = serde_json::from_value(json_value)
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

        self.init(typed_config, bar_config)
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
    }

    fn update(&mut self, event: Event) -> UpdateAction {
        self.update(event)
    }

    fn view(&self, pixmap: &mut PixmapMut, context: &mut RenderContext, monitor: &str) {
        self.view(pixmap, context, monitor)
    }

    fn measure(&self, context: &mut RenderContext, monitor: &str) -> f32 {
        self.measure(context, monitor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    Left,
    Center,
    Right,
}

pub struct ModuleRegistry {
    left_modules: Vec<Box<dyn AnyModule>>,
    center_modules: Vec<Box<dyn AnyModule>>,
    right_modules: Vec<Box<dyn AnyModule>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            left_modules: Vec::new(),
            center_modules: Vec::new(),
            right_modules: Vec::new(),
        }
    }

    /// Tell the registry to update all modules.
    /// Returns Redraw if any module requested a redraw.
    pub fn update(&mut self, event: Event) -> UpdateAction {
        let mut redraw = UpdateAction::None;

        for module in self
            .left_modules
            .iter_mut()
            .chain(self.center_modules.iter_mut())
            .chain(self.right_modules.iter_mut())
        {
            if module.update(event.clone()) == UpdateAction::Redraw {
                redraw = UpdateAction::Redraw;
            }
        }

        redraw
    }

    pub fn update_at(&mut self, pos: Position, index: usize, event: Event) -> UpdateAction {
        let modules = match pos {
            Position::Left => &mut self.left_modules,
            Position::Center => &mut self.center_modules,
            Position::Right => &mut self.right_modules,
        };

        if let Some(module) = modules.get_mut(index) {
            module.update(event)
        } else {
            UpdateAction::None
        }
    }

    pub fn left_modules(&self) -> &[Box<dyn AnyModule>] {
        &self.left_modules
    }

    pub fn center_modules(&self) -> &[Box<dyn AnyModule>] {
        &self.center_modules
    }

    pub fn right_modules(&self) -> &[Box<dyn AnyModule>] {
        &self.right_modules
    }

    pub fn load(&mut self, config: &crate::config::Config) -> RegistryResult<()> {
        if !Self::has_enabled_applet(config) {
            applet::drop_global_watcher();
        }

        self.left_modules = self.create_modules(config.modules().left(), config.bar())?;
        self.center_modules = self.create_modules(config.modules().center(), config.bar())?;
        self.right_modules = self.create_modules(config.modules().right(), config.bar())?;
        Ok(())
    }

    fn has_enabled_applet(config: &crate::config::Config) -> bool {
        config
            .modules()
            .left()
            .iter()
            .chain(config.modules().center().iter())
            .chain(config.modules().right().iter())
            .any(|module| module.is_enabled() && module.name() == "applet")
    }

    fn create_modules(
        &self,
        configs: &[ModuleConfig],
        bar_config: &crate::config::BarConfig,
    ) -> RegistryResult<Vec<Box<dyn AnyModule>>> {
        let mut modules = Vec::new();
        for config in configs {
            if !config.is_enabled() {
                continue;
            }

            let mut module: Box<dyn AnyModule> = match config.name() {
                "hour" => Box::new(hour::HourModule::new()),
                "applet" => Box::new(applet::AppletModule::new()),
                "workspace" => Box::new(workspace::WorkspaceModule::new()),
                name => return Err(RegistryError::NotFound(name.to_string())),
            };

            module
                .init(config, bar_config)
                .map_err(|e| RegistryError::InitFailed(config.name().to_string(), e))?;
            modules.push(module);
        }
        Ok(modules)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::collections::HashMap;

    #[test]
    fn test_registry_load() {
        let mut registry = ModuleRegistry::new();
        let full_toml = r##"
            [bar]
            background = "#1a1b26"
            border_radius = 4.0
            height = 30

            [[modules.left]]
            name = "hour"
            enable = true
            format = "%H:%M"

            [[modules.right]]
            name = "applet"
            enable = true
        "##;
        let config = Config::from_str(full_toml).unwrap();

        registry.load(&config).unwrap();

        assert_eq!(registry.left_modules.len(), 1);
        assert_eq!(registry.right_modules.len(), 1);
    }

    #[test]
    fn test_registry_not_found() {
        let mut registry = ModuleRegistry::new();
        let full_toml = r##"
            [bar]
            background = "#1a1b26"
            border_radius = 4.0
            height = 30

            [[modules.left]]
            name = "unknown"
            enable = true
        "##;
        let config = Config::from_str(full_toml).unwrap();

        let result = registry.load(&config);
        assert!(matches!(result, Err(RegistryError::NotFound(_))));
    }

    #[test]
    fn test_registry_update() {
        let mut registry = ModuleRegistry::new();
        let full_toml = r##"
            [bar]
            background = "#1a1b26"
            border_radius = 4.0
            height = 30

            [[modules.left]]
            name = "hour"
            enable = true
        "##;
        let config = Config::from_str(full_toml).unwrap();
        registry.load(&config).unwrap();

        let action = registry.update(Event::Timer);
        assert!(action == UpdateAction::Redraw || action == UpdateAction::None);
    }

    struct MockModule {
        width: f32,
        update_action: UpdateAction,
    }

    #[derive(serde::Deserialize, Default)]
    struct MockConfig {}

    impl CrankyModule for MockModule {
        type Error = std::io::Error;
        type Config = MockConfig;

        fn init(
            &mut self,
            _config: Self::Config,
            _bar_config: &crate::config::BarConfig,
        ) -> std::result::Result<(), Self::Error> {
            Ok(())
        }
        fn update(&mut self, _event: Event) -> UpdateAction {
            self.update_action.clone()
        }
        fn view(
            &self,
            _pixmap: &mut PixmapMut,
            _context: &mut RenderContext,
            _monitor: &str,
        ) {
        }
        fn measure(&self, _context: &mut RenderContext, _monitor: &str) -> f32 {
            self.width
        }
    }

    #[test]
    fn test_update_action_event() {
        let action = UpdateAction::Redraw;
        assert_eq!(action, UpdateAction::Redraw);
        assert_ne!(action, UpdateAction::None);

        let event = Event::Timer;
        let event_clone = event.clone();
        match event_clone {
            Event::Timer => {}
            Event::HyprlandUpdate { .. } => {}
            _ => {}
        }
    }

    #[test]
    fn test_registry_error_display() {
        let err = RegistryError::NotFound("test".to_string());
        assert_eq!(format!("{}", err), "Module not found: test");

        let sub_err = std::io::Error::new(std::io::ErrorKind::Other, "io error");
        let err = RegistryError::InitFailed("test".to_string(), Box::new(sub_err));
        assert!(format!("{}", err).contains("Module initialization failed for 'test'"));
    }

    #[test]
    fn test_registry_update_at() {
        let mut registry = ModuleRegistry::new();
        let mock = MockModule {
            width: 100.0,
            update_action: UpdateAction::Redraw,
        };
        registry.left_modules.push(Box::new(mock));

        let action = registry.update_at(Position::Left, 0, Event::Timer);
        assert_eq!(action, UpdateAction::Redraw);

        let action = registry.update_at(Position::Left, 1, Event::Timer); // Out of bounds
        assert_eq!(action, UpdateAction::None);
    }

    #[test]
    fn test_registry_update_at_positions() {
        let mut registry = ModuleRegistry::new();
        registry.left_modules.push(Box::new(MockModule {
            width: 10.0,
            update_action: UpdateAction::Redraw,
        }));
        registry.center_modules.push(Box::new(MockModule {
            width: 20.0,
            update_action: UpdateAction::None,
        }));
        registry.right_modules.push(Box::new(MockModule {
            width: 30.0,
            update_action: UpdateAction::Redraw,
        }));

        assert_eq!(
            registry.update_at(Position::Left, 0, Event::Timer),
            UpdateAction::Redraw
        );
        assert_eq!(
            registry.update_at(Position::Center, 0, Event::Timer),
            UpdateAction::None
        );
        assert_eq!(
            registry.update_at(Position::Right, 0, Event::Timer),
            UpdateAction::Redraw
        );
    }

    #[test]
    fn test_any_module_trait() {
        let mut mock = MockModule {
            width: 100.0,
            update_action: UpdateAction::None,
        };

        // Testing that AnyModule methods (which are implemented via the generic impl) work
        let event_action = AnyModule::update(&mut mock, Event::Timer);
        assert_eq!(event_action, UpdateAction::None);

        let mut context = RenderContext::new();
        let width = AnyModule::measure(&mock, &mut context, "eDP-1");
        assert_eq!(width, 100.0);
    }

    #[test]
    fn test_registry_update_redraw() {
        let mut registry = ModuleRegistry::new();
        let mock = MockModule {
            width: 100.0,
            update_action: UpdateAction::Redraw,
        };
        registry.left_modules.push(Box::new(mock));

        let action = registry.update(Event::Timer);
        assert_eq!(action, UpdateAction::Redraw);
    }

    #[test]
    fn test_registry_create_modules_disabled() {
        let registry = ModuleRegistry::new();
        let options = HashMap::new();
        let configs = vec![
            ModuleConfig::new("hour".to_string(), false, options.clone()), // Disabled
            ModuleConfig::new("hour".to_string(), true, options),          // Enabled
        ];
        let bar_config = crate::config::BarConfig::default();

        let modules = registry.create_modules(&configs, &bar_config).unwrap();
        assert_eq!(modules.len(), 1); // Only the enabled one
    }
}
