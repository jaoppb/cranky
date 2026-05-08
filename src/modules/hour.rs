use crate::modules::CrankyModule;
use crate::ports::canvas::{Canvas, Color as CanvasColor};
use crate::domain::signals::SignalHub;
use crate::domain::errors::DomainError;
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

impl<C: Canvas> CrankyModule<C> for HourModule {
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

    fn view(&self, canvas: &mut C, _monitor: &str) {
        let color: CanvasColor = (192, 202, 245, 255);
        canvas.draw_text(&self.current_time, "", 14.0, color, 0.0, 15.0);
    }

    fn measure(&self, canvas: &mut C, _monitor: &str) -> (f32, f32) {
        canvas.measure_text(&self.current_time, "", 14.0)
    }
}
