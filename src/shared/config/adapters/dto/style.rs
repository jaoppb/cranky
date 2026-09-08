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
        let mut config = domain::TooltipConfig::default();
        if let Some(bg) = self.background.and_then(|c| DrawingColor::parse(&c).ok()) {
            config = config.with_background(bg);
        }
        if let Some(bc) = self.border_color.and_then(|c| DrawingColor::parse(&c).ok()) {
            config = config.with_border_color(bc);
        }
        if let Some(tc) = self.text_color.and_then(|c| DrawingColor::parse(&c).ok()) {
            config = config.with_text_color(tc);
        }
        if let Some(font) = self.font {
            config = config.with_font(Some(FontFamily::new(font)));
        }
        if let Some(size) = self.size {
            config = config.with_size(Some(FontSize::new(size)));
        }
        if let Some(radius) = self.radius {
            config = config.with_radius(BorderRadius::new(radius));
        }
        if let Some(border_width) = self.border_width {
            config = config.with_border_width(BorderSize::new(border_width));
        }
        if let Some(padding) = self.padding {
            config = config.with_padding(PaddingOffset::new(padding));
        }
        config
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct PopupConfigDto {
    #[serde(default)]
    behavior: PopupBehaviorDto,
    #[serde(default)]
    offset: Option<crate::shared::primitives::PopupOffset>,
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
        let offset = match self.offset {
            Some(off) => off,
            None => crate::shared::primitives::PopupOffset::new(0, 8),
        };
        domain::PopupConfig::new(self.behavior.into_domain(), offset)
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
