use crate::modules::{CrankyModule, Event, UpdateAction};
use crate::render::RenderContext;
use chrono::Local;
use serde::Deserialize;
use thiserror::Error;
use tiny_skia::{Color, PixmapMut, Rect};

#[derive(Error, Debug)]
pub enum HourError {}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct HourConfig {
    #[serde(default)]
    pub format: Option<String>,
}

pub struct HourModule {
    format: String,
    current_time: String,
    font_family: String,
}

impl HourModule {
    pub fn new() -> Self {
        Self {
            format: "%H:%M:%S".to_string(),
            current_time: String::new(),
            font_family: String::new(),
        }
    }
}

impl CrankyModule for HourModule {
    type Error = HourError;
    type Config = HourConfig;

    fn init(
        &mut self,
        config: Self::Config,
        _bar_config: &crate::config::BarConfig,
    ) -> Result<(), Self::Error> {
        if let Some(format) = config.format {
            self.format = format;
        }
        self.current_time = Local::now().format(&self.format).to_string();
        Ok(())
    }

    fn update(&mut self, event: Event) -> UpdateAction {
        match event {
            Event::Timer => {
                let new_time = Local::now().format(&self.format).to_string();
                if new_time != self.current_time {
                    self.current_time = new_time;
                    UpdateAction::Redraw
                } else {
                    UpdateAction::None
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
        use crate::render::TextStyling;
        let styling = TextStyling::new(
            14.0,
            20.0,
            Color::from_rgba8(192, 202, 245, 255),
            self.font_family.clone(),
        );

        let y_offset = context.calculate_vertical_offset(area, styling.line_height());

        context.render_text(pixmap, &self.current_time, styling, area.left(), y_offset);
    }

    fn measure(&self, context: &mut RenderContext, _monitor: &str) -> f32 {
        use crate::render::TextStyling;
        context.measure_text(
            &self.current_time,
            TextStyling::new(
                14.0,
                20.0,
                Color::from_rgba8(192, 202, 245, 255),
                self.font_family.clone(),
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hour_init() {
        let mut module = HourModule::new();
        let config = HourConfig {
            format: Some("%H:%M".to_string()),
        };

        let bar_toml = r##"
            background = "#1a1b26"
            border_radius = 4.0
            height = 30
        "##;
        let bar_config: crate::config::BarConfig = toml::from_str(bar_toml).unwrap();

        module.init(config, &bar_config).unwrap();
        assert_eq!(module.format, "%H:%M");
    }

    #[test]
    fn test_hour_update() {
        let mut module = HourModule::new();
        let config = HourConfig::default();
        let bar_config = crate::config::BarConfig::default();
        module.init(config, &bar_config).unwrap();

        let action = module.update(Event::Timer);
        // It could be either None or Redraw depending on when it's called, but it shouldn't panic.
        match action {
            UpdateAction::Redraw | UpdateAction::None => {}
        }
    }

    #[test]
    fn test_hour_view_measure() {
        let mut module = HourModule::new();
        let config = HourConfig::default();
        let bar_config = crate::config::BarConfig::default();
        module.init(config, &bar_config).unwrap();

        let mut pixmap_data = vec![0; 100 * 30 * 4];
        let mut pixmap = PixmapMut::from_bytes(&mut pixmap_data, 100, 30).unwrap();
        let mut context = RenderContext::new();
        let area = Rect::from_xywh(0.0, 0.0, 100.0, 30.0).unwrap();

        module.view(&mut pixmap, area, &mut context, "eDP-1");
        let width = module.measure(&mut context, "eDP-1");
        assert!(width > 0.0);
    }
}
