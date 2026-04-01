use crate::config::ModuleConfig;
use crate::render::RenderContext;
use log::info;
use std::error::Error;
use thiserror::Error;
use tiny_skia::PixmapMut;

pub mod hour;
#[macro_use]
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
    fn view(
        &self,
        pixmap: &mut PixmapMut,
        area: tiny_skia::Rect,
        context: &mut RenderContext,
        monitor: &str,
    );

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
    fn view(
        &self,
        pixmap: &mut PixmapMut,
        area: tiny_skia::Rect,
        context: &mut RenderContext,
        monitor: &str,
    );
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

    fn view(
        &self,
        pixmap: &mut PixmapMut,
        area: tiny_skia::Rect,
        context: &mut RenderContext,
        monitor: &str,
    ) {
        self.view(pixmap, area, context, monitor)
    }

    fn measure(&self, context: &mut RenderContext, monitor: &str) -> f32 {
        self.measure(context, monitor)
    }
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

    pub fn view(
        &self,
        pixmap: &mut PixmapMut,
        area: tiny_skia::Rect,
        context: &mut RenderContext,
        monitor: &str,
    ) {
        let spacing = 10.0;
        info!(
            "ModuleRegistry::view: area={:?}, scale={}",
            area,
            context.scale()
        );

        // Render left modules
        let mut left_offset = area.left() + spacing;
        for module in &self.left_modules {
            let width = module.measure(context, monitor);
            let module_area =
                tiny_skia::Rect::from_xywh(left_offset, area.top(), width, area.height()).unwrap();
            module.view(pixmap, module_area, context, monitor);
            left_offset += width + spacing;
        }

        // Render right modules
        let mut right_widths = Vec::new();
        let mut total_right_width = 0.0;
        for module in &self.right_modules {
            let width = module.measure(context, monitor);
            right_widths.push(width);
            total_right_width += width + spacing;
        }

        let mut right_offset = area.right() - total_right_width;
        for (i, module) in self.right_modules.iter().enumerate() {
            let width = right_widths[i];
            let module_area =
                tiny_skia::Rect::from_xywh(right_offset, area.top(), width, area.height()).unwrap();
            module.view(pixmap, module_area, context, monitor);
            right_offset += width + spacing;
        }

        // Render center modules
        let mut center_widths = Vec::new();
        let mut total_center_width = 0.0;
        for module in &self.center_modules {
            let width = module.measure(context, monitor);
            center_widths.push(width);
            total_center_width += width + spacing;
        }
        // Remove the last spacing from total width for centering calculation
        if !center_widths.is_empty() {
            total_center_width -= spacing;
        }

        let mut center_offset = area.left() + (area.width() - total_center_width) / 2.0;
        for (i, module) in self.center_modules.iter().enumerate() {
            let width = center_widths[i];
            let module_area =
                tiny_skia::Rect::from_xywh(center_offset, area.top(), width, area.height())
                    .unwrap();
            module.view(pixmap, module_area, context, monitor);
            center_offset += width + spacing;
        }
    }

    pub fn load(&mut self, config: &crate::config::Config) -> RegistryResult<()> {
        self.left_modules = self.create_modules(config.modules().left(), config.bar())?;
        self.center_modules = self.create_modules(config.modules().center(), config.bar())?;
        self.right_modules = self.create_modules(config.modules().right(), config.bar())?;
        Ok(())
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
        "##;
        let config = Config::from_str(full_toml).unwrap();

        registry.load(&config).unwrap();

        assert_eq!(registry.left_modules.len(), 1);
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

    use std::sync::{Arc, Mutex};

    struct MockModule {
        width: f32,
        areas: Arc<Mutex<Vec<tiny_skia::Rect>>>,
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
            area: tiny_skia::Rect,
            _context: &mut RenderContext,
            _monitor: &str,
        ) {
            self.areas.lock().unwrap().push(area);
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
    fn test_any_module_trait() {
        let mut mock = MockModule {
            width: 100.0,
            areas: Arc::new(Mutex::new(Vec::new())),
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
            areas: Arc::new(Mutex::new(Vec::new())),
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

    #[test]
    fn test_registry_view_layout() {
        let mut registry = ModuleRegistry::new();
        let areas = Arc::new(Mutex::new(Vec::new()));

        let m1 = Box::new(MockModule {
            width: 40.0,
            areas: areas.clone(),
            update_action: UpdateAction::None,
        });
        let m2 = Box::new(MockModule {
            width: 60.0,
            areas: areas.clone(),
            update_action: UpdateAction::None,
        });
        let m3 = Box::new(MockModule {
            width: 50.0,
            areas: areas.clone(),
            update_action: UpdateAction::None,
        });


        registry.left_modules.push(m1);
        registry.center_modules.push(m2);
        registry.right_modules.push(m3);

        let mut pixmap_data = vec![0; 1000 * 30 * 4];
        let mut pixmap = PixmapMut::from_bytes(&mut pixmap_data, 1000, 30).unwrap();
        let area = tiny_skia::Rect::from_xywh(0.0, 0.0, 1000.0, 30.0).unwrap();
        let mut context = RenderContext::new();

        registry.view(&mut pixmap, area, &mut context, "eDP-1");

        let rendered_areas = areas.lock().unwrap();
        assert_eq!(rendered_areas.len(), 3);

        // Left module (m1)
        // Offset: area.left() + 10.0 = 10.0
        // Width: 40.0
        assert_eq!(rendered_areas[0].left(), 10.0);
        assert_eq!(rendered_areas[0].width(), 40.0);

        // Right module (m3)
        // Total right width: 50.0 + 10.0 = 60.0
        // Offset: 1000.0 - 60.0 = 940.0
        assert_eq!(rendered_areas[1].left(), 940.0);
        assert_eq!(rendered_areas[1].width(), 50.0);

        // Center module (m2)
        // Total center width: 60.0
        // Offset: 0.0 + (1000.0 - 60.0) / 2.0 = 470.0
        assert_eq!(rendered_areas[2].left(), 470.0);
        assert_eq!(rendered_areas[2].width(), 60.0);

    }
}
