use crate::modules::CrankyModule;
use crate::ports::canvas::{Canvas};
use crate::domain::signals::SignalHub;
use crate::domain::errors::DomainError;
use crate::domain::color::DrawingColor;
use serde::Deserialize;

#[derive(Debug, Deserialize, Default, Clone)]
pub struct HourConfig {
    #[serde(default)]
    format: Option<String>,
}

pub struct HourModule {
    format: String,
    current_time: String,
}

impl HourModule {
    pub fn new() -> Self {
        Self {
            format: "%H:%M:%S".to_string(),
            current_time: String::new(),
        }
    }
}

impl CrankyModule for HourModule {
    type Config = HourConfig;

    fn init(
        &mut self,
        config: Self::Config,
        _bar_config: &crate::config::BarConfig,
    ) -> Result<(), DomainError> {
        if let Some(format) = config.format {
            self.format = format;
        }
        self.current_time = chrono::Local::now().format(&self.format).to_string();
        Ok(())
    }

    fn attach(&mut self, hub: &SignalHub, target_id: u32) {
        let mut time_rx = hub.time_rx();
        let dirty_tx = hub.dirty_tx();
        
        tokio::spawn(async move {
            while time_rx.changed().await.is_ok() {
                let _ = dirty_tx.send(target_id).await;
            }
        });
    }

    fn refresh(&mut self, hub: &SignalHub) {
        let time = *hub.time_rx().borrow();
        self.current_time = time.format(&self.format).to_string();
    }

    fn view(&self, canvas: &mut dyn Canvas, _monitor: &str) {
        let color: DrawingColor = DrawingColor::parse("#c0caf5").unwrap();
        canvas.draw_text(&self.current_time, "", 14.0, color, 0.0, 15.0);
    }

    fn measure(&self, canvas: &mut dyn Canvas, _monitor: &str) -> (f32, f32) {
        canvas.measure_text(&self.current_time, "", 14.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::canvas::MockCanvas;
    use crate::config::Config;
    use crate::modules::CrankyModule;

    #[test]
    fn test_hour_module_init() {
        let mut module = HourModule::new();
        let config = HourConfig { format: Some("%H".to_string()) };
        CrankyModule::init(&mut module, config, &crate::config::BarConfig::default()).unwrap();
        assert_eq!(module.format, "%H");
        assert!(module.current_time.len() <= 2);
    }

    #[test]
    fn test_hour_module_view() {
        let mut module = HourModule::new();
        module.current_time = "12:34:56".to_string();
        
        let mut mock = MockCanvas::new();
        mock.expect_draw_text()
            .withf(|text, font, size, _color, x, y| {
                text == "12:34:56" && font == "" && *size == 14.0 && *x == 0.0 && *y == 15.0
            })
            .times(1)
            .returning(|_, _, _, _, _, _| ());

        CrankyModule::view(&module, &mut mock, "eDP-1");
    }

    #[tokio::test]
    async fn test_hour_module_reactive_dirty() {
        let (hub, mut dirty_rx) = SignalHub::new(Config::default());
        let mut module = HourModule::new();
        
        // target_id = 42
        CrankyModule::attach(&mut module, &hub, 42);

        // Simulate time change
        hub.time_tx().send(chrono::Local::now()).unwrap();

        let id = dirty_rx.recv().await.unwrap();
        assert_eq!(id, 42);
    }
}
