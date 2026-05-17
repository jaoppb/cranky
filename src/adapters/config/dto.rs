use serde::Deserialize;
use std::collections::HashMap;
use crate::domain::config as domain;
use crate::domain::color::DrawingColor;
use crate::ports::font::FontValidatorPort;

#[derive(Debug, Deserialize)]
pub struct ConfigDto {
    #[serde(default)]
    font_family: Option<String>,
    #[serde(default)]
    font_size: Option<f32>,
    bar: BarConfigDto,
    #[serde(default)]
    modules: ModulesConfigDto,
    #[serde(default)]
    rendering: RenderingModeDto,
}

impl ConfigDto {
    pub fn to_domain<V: FontValidatorPort>(self, validator: &V) -> domain::Config {
        let font_family = self.font_family
            .filter(|f| validator.is_valid_family(f))
            .unwrap_or_else(|| "".to_string()); // Fallback to empty string for rendering defaults if invalid
            
        let font_size = self.font_size.unwrap_or(14.0);

        let bar = self.bar.to_domain(
            domain::FontFamily::new(font_family),
            domain::FontSize::new(font_size),
        );
        let modules = self.modules.to_domain();
        let rendering = self.rendering.to_domain();

        domain::Config::new(bar, modules, rendering)
    }
}

#[derive(Debug, Deserialize)]
pub struct BarConfigDto {
    #[serde(default = "default_background")]
    background: DrawingColor,
    #[serde(default = "default_height")]
    height: u32,
    #[serde(default)]
    vertical_alignment: VerticalAlignmentDto,
    #[serde(default)]
    border: BorderConfigDto,
    #[serde(default)]
    margin: MarginConfigDto,
    #[serde(default)]
    unfocused: Option<PartialBarConfigDto>,
}

impl BarConfigDto {
    pub fn to_domain(
        self,
        font_family: domain::FontFamily,
        font_size: domain::FontSize,
    ) -> domain::BarConfig {
        domain::BarConfig::new(
            self.background,
            self.height,
            self.vertical_alignment.to_domain(),
            self.border.to_domain(),
            self.margin.to_domain(),
            font_family,
            font_size,
            self.unfocused.map(|u| u.to_domain()),
        )
    }
}

fn default_background() -> DrawingColor {
    DrawingColor::Solid(crate::domain::color::Color::new(0, 0, 0, 255))
}

fn default_height() -> u32 {
    30
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum VerticalAlignmentDto {
    Top,
    #[default]
    Center,
    Bottom,
}

impl VerticalAlignmentDto {
    pub fn to_domain(self) -> domain::VerticalAlignment {
        match self {
            VerticalAlignmentDto::Top => domain::VerticalAlignment::Top,
            VerticalAlignmentDto::Center => domain::VerticalAlignment::Center,
            VerticalAlignmentDto::Bottom => domain::VerticalAlignment::Bottom,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct MarginConfigDto {
    #[serde(default)]
    top: i32,
    #[serde(default)]
    bottom: i32,
    #[serde(default)]
    left: i32,
    #[serde(default)]
    right: i32,
}

impl MarginConfigDto {
    pub fn to_domain(self) -> domain::MarginConfig {
        domain::MarginConfig::new(self.top, self.bottom, self.left, self.right)
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct BorderConfigDto {
    #[serde(default)]
    size: f32,
    #[serde(default = "default_border_color")]
    color: DrawingColor,
    #[serde(default)]
    radius: f32,
}

impl BorderConfigDto {
    pub fn to_domain(self) -> domain::BorderConfig {
        domain::BorderConfig::new(self.size, self.color, self.radius)
    }
}

fn default_border_color() -> DrawingColor {
    DrawingColor::Solid(crate::domain::color::Color::new(0, 0, 0, 255))
}

#[derive(Debug, Deserialize, Default)]
pub struct ModulesConfigDto {
    #[serde(default)]
    left: Vec<ModuleConfigDto>,
    #[serde(default)]
    center: Vec<ModuleConfigDto>,
    #[serde(default)]
    right: Vec<ModuleConfigDto>,
}

impl ModulesConfigDto {
    pub fn to_domain(self) -> domain::ModulesConfig {
        domain::ModulesConfig::new(
            self.left.into_iter().map(|m| m.to_domain()).collect(),
            self.center.into_iter().map(|m| m.to_domain()).collect(),
            self.right.into_iter().map(|m| m.to_domain()).collect(),
        )
    }
}

#[derive(Debug, Deserialize)]
pub struct ModuleConfigDto {
    name: String,
    enable: bool,
    #[serde(flatten)]
    options: HashMap<String, serde_json::Value>,
}

impl ModuleConfigDto {
    pub fn to_domain(self) -> domain::ModuleConfig {
        domain::ModuleConfig::new(self.name, self.enable, self.options)
    }
}

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
    pub fn to_domain(self) -> domain::RenderingMode {
        match self {
            RenderingModeDto::Immediate { fps_limit } => domain::RenderingMode::new_immediate(fps_limit),
            RenderingModeDto::Timebased { duration_ms } => domain::RenderingMode::new_timebased(duration_ms),
        }
    }
}

fn default_timebased_duration_ms() -> u64 {
    100
}

#[derive(Debug, Deserialize, Default)]
pub struct PartialMarginConfigDto {
    pub top: Option<i32>,
    pub bottom: Option<i32>,
    pub left: Option<i32>,
    pub right: Option<i32>,
}

impl PartialMarginConfigDto {
    pub fn to_domain(self) -> domain::PartialMarginConfig {
        domain::PartialMarginConfig {
            top: self.top,
            bottom: self.bottom,
            left: self.left,
            right: self.right,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct PartialBorderConfigDto {
    pub size: Option<f32>,
    pub color: Option<DrawingColor>,
    pub radius: Option<f32>,
}

impl PartialBorderConfigDto {
    pub fn to_domain(self) -> domain::PartialBorderConfig {
        domain::PartialBorderConfig {
            size: self.size,
            color: self.color,
            radius: self.radius,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct PartialBarConfigDto {
    pub background: Option<DrawingColor>,
    pub height: Option<u32>,
    pub vertical_alignment: Option<VerticalAlignmentDto>,
    pub border: Option<PartialBorderConfigDto>,
    pub margin: Option<PartialMarginConfigDto>,
}

impl PartialBarConfigDto {
    pub fn to_domain(self) -> domain::PartialBarConfig {
        domain::PartialBarConfig {
            background: self.background,
            height: self.height,
            vertical_alignment: self.vertical_alignment.map(|v| v.to_domain()),
            border: self.border.map(|b| b.to_domain()),
            margin: self.margin.map(|m| m.to_domain()),
        }
    }
}
