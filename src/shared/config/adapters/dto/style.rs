use serde::Deserialize;

use crate::shared::config::domain::{self, BorderRadius, BorderSize, FontFamily, FontSize, PaddingOffset};
use crate::shared::primitives::color::DrawingColor;

use super::helpers::default_timebased_duration_ms;

#[derive(Debug, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum RenderingModeDto {
    Immediate {
        #[serde(default)]
        fps_limit: Option<u32>,
    },
    Timebased {
        #[serde(default = "default_timebased_duration_ms")]
        duration_ms: u64,
    },
}

impl Default for RenderingModeDto {
    fn default() -> Self {
        Self::Timebased {
            duration_ms: default_timebased_duration_ms(),
        }
    }
}

impl RenderingModeDto {
    #[must_use]
    pub const fn into_domain(self) -> domain::RenderingMode {
        match self {
            Self::Immediate { fps_limit } => domain::RenderingMode::new_immediate(fps_limit),
            Self::Timebased { duration_ms } => domain::RenderingMode::new_timebased(duration_ms),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct TooltipConfigDto {
    background: Option<String>,
    border_color: Option<String>,
    text_color: Option<String>,
    font: Option<String>,
    size: Option<f32>,
    radius: Option<f32>,
    border_width: Option<f32>,
    padding: Option<u32>,
}

impl TooltipConfigDto {
    #[must_use]
    pub fn into_domain(self) -> domain::TooltipConfig {
        let default = domain::TooltipConfig::default();
        domain::TooltipConfig::new(
            self.background
                .and_then(|c| DrawingColor::parse(&c).ok())
                .unwrap_or_else(|| default.background().clone()),
            self.border_color
                .and_then(|c| DrawingColor::parse(&c).ok())
                .unwrap_or_else(|| default.border_color().clone()),
            self.text_color
                .and_then(|c| DrawingColor::parse(&c).ok())
                .unwrap_or_else(|| default.text_color().clone()),
            self.font
                .map(FontFamily::new)
                .or_else(|| default.font().cloned()),
            self.size.map(FontSize::new).or_else(|| default.size()),
            self.radius
                .map_or_else(|| default.radius(), BorderRadius::new),
            self.border_width
                .map_or_else(|| default.border_width(), BorderSize::new),
            self.padding
                .map_or_else(|| default.padding(), PaddingOffset::new),
        )
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct PopupConfigDto {
    #[serde(default)]
    behavior: PopupBehaviorDto,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PopupBehaviorDto {
    PerMonitor,
    PerModule,
    PerModuleAndMonitor,
    #[default]
    Global,
}

impl PopupConfigDto {
    #[must_use]
    pub const fn into_domain(self) -> domain::PopupConfig {
        domain::PopupConfig::new(self.behavior.into_domain())
    }
}

impl PopupBehaviorDto {
    #[must_use]
    pub const fn into_domain(self) -> domain::PopupBehavior {
        match self {
            Self::PerMonitor => domain::PopupBehavior::PerMonitor,
            Self::PerModule => domain::PopupBehavior::PerModule,
            Self::PerModuleAndMonitor => domain::PopupBehavior::PerModuleAndMonitor,
            Self::Global => domain::PopupBehavior::Global,
        }
    }
}
