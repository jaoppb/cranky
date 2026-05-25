use serde::Deserialize;
use std::collections::HashMap;
use crate::domain::config as domain;
use crate::domain::color::DrawingColor;
use crate::ports::font::FontValidatorPort;

#[derive(Debug, Deserialize)]
pub struct ConfigDto {
    bar: BarConfigDto,
    #[serde(default)]
    modules: ModulesConfigDto,
    #[serde(default)]
    rendering: RenderingModeDto,
    #[serde(default)]
    metrics: crate::domain::metrics::MetricsConfig,
}

impl ConfigDto {
    pub fn to_domain<V: FontValidatorPort>(self, validator: &V) -> domain::Config {
        let bar = self.bar.to_domain(validator);
        let modules = self.modules.to_domain();
        let rendering = self.rendering.to_domain();

        domain::Config::new(bar, modules, rendering, self.metrics)
    }
}
#[derive(Debug, Deserialize)]
pub struct BarConfigDto {
    #[serde(default)]
    font_family: Option<String>,
    #[serde(default)]
    font_size: Option<f32>,
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
    padding: PaddingConfigDto,
    #[serde(default)]
    unfocused: Option<PartialBarConfigDto>,
}

impl BarConfigDto {
    pub fn to_domain<V: FontValidatorPort>(
        self,
        validator: &V,
    ) -> domain::BarConfig {
        let font_family = self.font_family
            .filter(|f| validator.is_valid_family(f))
            .unwrap_or_else(|| "".to_string());
            
        let font_size = self.font_size.unwrap_or(14.0);

        domain::BarConfig::new(
            self.background,
            self.height,
            self.vertical_alignment.to_domain(),
            self.border.to_domain(),
            self.margin.to_domain(),
            self.padding.to_domain(),
            domain::FontFamily::new(font_family),
            domain::FontSize::new(font_size),
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

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum MarginConfigDto {
    All(i32),
    Fields {
        top: Option<i32>,
        bottom: Option<i32>,
        left: Option<i32>,
        right: Option<i32>,
        horizontal: Option<i32>,
        vertical: Option<i32>,
    }
}

impl Default for MarginConfigDto {
    fn default() -> Self {
        Self::All(0)
    }
}

impl MarginConfigDto {
    pub fn to_domain(self) -> domain::MarginConfig {
        match self {
            Self::All(val) => domain::MarginConfig::new(
                domain::MarginOffset::new(val),
                domain::MarginOffset::new(val),
                domain::MarginOffset::new(val),
                domain::MarginOffset::new(val),
            ),
            Self::Fields { top, bottom, left, right, horizontal, vertical } => {
                let t = top.or(vertical).unwrap_or(0);
                let b = bottom.or(vertical).unwrap_or(0);
                let l = left.or(horizontal).unwrap_or(0);
                let r = right.or(horizontal).unwrap_or(0);
                domain::MarginConfig::new(
                    domain::MarginOffset::new(t),
                    domain::MarginOffset::new(b),
                    domain::MarginOffset::new(l),
                    domain::MarginOffset::new(r),
                )
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PaddingConfigDto {
    All(u32),
    Fields {
        top: Option<u32>,
        bottom: Option<u32>,
        left: Option<u32>,
        right: Option<u32>,
        horizontal: Option<u32>,
        vertical: Option<u32>,
    }
}

impl Default for PaddingConfigDto {
    fn default() -> Self {
        Self::All(0)
    }
}

impl PaddingConfigDto {
    pub fn to_domain(self) -> domain::PaddingConfig {
        match self {
            Self::All(val) => domain::PaddingConfig::new(
                domain::PaddingOffset::new(val),
                domain::PaddingOffset::new(val),
                domain::PaddingOffset::new(val),
                domain::PaddingOffset::new(val),
            ),
            Self::Fields { top, bottom, left, right, horizontal, vertical } => {
                let t = top.or(vertical).unwrap_or(0);
                let b = bottom.or(vertical).unwrap_or(0);
                let l = left.or(horizontal).unwrap_or(0);
                let r = right.or(horizontal).unwrap_or(0);
                domain::PaddingConfig::new(
                    domain::PaddingOffset::new(t),
                    domain::PaddingOffset::new(b),
                    domain::PaddingOffset::new(l),
                    domain::PaddingOffset::new(r),
                )
            }
        }
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
        domain::BorderConfig::new(
            domain::BorderSize::new(self.size),
            self.color,
            domain::BorderRadius::new(self.radius),
        )
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

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PartialMarginConfigDto {
    All(i32),
    Fields {
        top: Option<i32>,
        bottom: Option<i32>,
        left: Option<i32>,
        right: Option<i32>,
        horizontal: Option<i32>,
        vertical: Option<i32>,
    }
}

impl Default for PartialMarginConfigDto {
    fn default() -> Self {
        Self::Fields {
            top: None, bottom: None, left: None, right: None, horizontal: None, vertical: None
        }
    }
}

impl PartialMarginConfigDto {
    pub fn to_domain(self) -> domain::PartialMarginConfig {
        match self {
            Self::All(val) => domain::PartialMarginConfig::new(
                Some(domain::MarginOffset::new(val)),
                Some(domain::MarginOffset::new(val)),
                Some(domain::MarginOffset::new(val)),
                Some(domain::MarginOffset::new(val)),
            ),
            Self::Fields { top, bottom, left, right, horizontal, vertical } => {
                let t = top.or(vertical).map(domain::MarginOffset::new);
                let b = bottom.or(vertical).map(domain::MarginOffset::new);
                let l = left.or(horizontal).map(domain::MarginOffset::new);
                let r = right.or(horizontal).map(domain::MarginOffset::new);
                domain::PartialMarginConfig::new(t, b, l, r)
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PartialPaddingConfigDto {
    All(u32),
    Fields {
        top: Option<u32>,
        bottom: Option<u32>,
        left: Option<u32>,
        right: Option<u32>,
        horizontal: Option<u32>,
        vertical: Option<u32>,
    }
}

impl Default for PartialPaddingConfigDto {
    fn default() -> Self {
        Self::Fields {
            top: None, bottom: None, left: None, right: None, horizontal: None, vertical: None
        }
    }
}

impl PartialPaddingConfigDto {
    pub fn to_domain(self) -> domain::PartialPaddingConfig {
        match self {
            Self::All(val) => domain::PartialPaddingConfig::new(
                Some(domain::PaddingOffset::new(val)),
                Some(domain::PaddingOffset::new(val)),
                Some(domain::PaddingOffset::new(val)),
                Some(domain::PaddingOffset::new(val)),
            ),
            Self::Fields { top, bottom, left, right, horizontal, vertical } => {
                let t = top.or(vertical).map(domain::PaddingOffset::new);
                let b = bottom.or(vertical).map(domain::PaddingOffset::new);
                let l = left.or(horizontal).map(domain::PaddingOffset::new);
                let r = right.or(horizontal).map(domain::PaddingOffset::new);
                domain::PartialPaddingConfig::new(t, b, l, r)
            }
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct PartialBorderConfigDto {
    size: Option<f32>,
    color: Option<DrawingColor>,
    radius: Option<f32>,
}

impl PartialBorderConfigDto {
    pub fn to_domain(self) -> domain::PartialBorderConfig {
        domain::PartialBorderConfig::new(
            self.size.map(domain::BorderSize::new),
            self.color,
            self.radius.map(domain::BorderRadius::new),
        )
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct PartialBarConfigDto {
    font_family: Option<String>,
    font_size: Option<f32>,
    background: Option<DrawingColor>,
    height: Option<u32>,
    vertical_alignment: Option<VerticalAlignmentDto>,
    border: Option<PartialBorderConfigDto>,
    margin: Option<PartialMarginConfigDto>,
    padding: Option<PartialPaddingConfigDto>,
}

impl PartialBarConfigDto {
    pub fn to_domain(self) -> domain::PartialBarConfig {
        domain::PartialBarConfig::new(
            self.background,
            self.height,
            self.vertical_alignment.map(|va| va.to_domain()),
            self.border.map(|b| b.to_domain()),
            self.margin.map(|m| m.to_domain()),
            self.padding.map(|p| p.to_domain()),
            self.font_family.map(domain::FontFamily::new),
            self.font_size.map(domain::FontSize::new),
        )
    }
}
