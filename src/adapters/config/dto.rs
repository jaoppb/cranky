use crate::domain::config as domain;
use crate::domain::shared::color::DrawingColor;
use crate::ports::font::FontValidatorPort;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct ConfigDto {
    bar: BarConfigDto,
    #[serde(default)]
    modules: ModulesConfigDto,
    #[serde(default)]
    rendering: RenderingModeDto,
    #[serde(default)]
    metrics: crate::domain::metrics::MetricsConfig,
    #[serde(default)]
    tooltip: TooltipConfigDto,
}

impl ConfigDto {
    pub fn into_domain<V: FontValidatorPort>(self, validator: &V) -> domain::Config {
        let bar = self.bar.into_domain(validator);
        let modules = self.modules.into_domain();
        let rendering = self.rendering.into_domain();
        let tooltip = self.tooltip.into_domain();

        domain::Config::new(bar, modules, rendering, self.metrics, tooltip)
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
    module_gap: u32,
    #[serde(default)]
    unfocused: Option<PartialBarConfigDto>,
}

impl BarConfigDto {
    pub fn into_domain<V: FontValidatorPort>(self, validator: &V) -> domain::BarConfig {
        let font_family = self
            .font_family
            .filter(|f| validator.is_valid_family(f))
            .unwrap_or_default();

        let font_size = self.font_size.unwrap_or(14.0);

        domain::BarConfig::new(crate::domain::config::CreateBarConfigCommand {
            background: self.background,
            height: crate::domain::shared::geometry::BarHeight::new(self.height),
            vertical_alignment: self.vertical_alignment.into_domain(),
            border: self.border.into_domain(),
            margin: self.margin.into_domain(),
            padding: self.padding.into_domain(),
            module_gap: domain::ModuleGap::new(self.module_gap),
            font_family: domain::FontFamily::new(font_family),
            font_size: domain::FontSize::new(font_size),
            unfocused: self.unfocused.map(|u| u.into_domain()),
        })
    }
}

fn default_background() -> DrawingColor {
    DrawingColor::Solid(crate::domain::shared::color::Color::new(0, 0, 0, 255))
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
    pub fn into_domain(self) -> domain::VerticalAlignment {
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
    },
}

impl Default for MarginConfigDto {
    fn default() -> Self {
        Self::All(0)
    }
}

impl MarginConfigDto {
    pub fn into_domain(self) -> domain::MarginConfig {
        match self {
            Self::All(val) => domain::MarginConfig::new(
                domain::MarginOffset::new(val),
                domain::MarginOffset::new(val),
                domain::MarginOffset::new(val),
                domain::MarginOffset::new(val),
            ),
            Self::Fields {
                top,
                bottom,
                left,
                right,
                horizontal,
                vertical,
            } => {
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
    },
}

impl Default for PaddingConfigDto {
    fn default() -> Self {
        Self::All(0)
    }
}

impl PaddingConfigDto {
    pub fn into_domain(self) -> domain::PaddingConfig {
        match self {
            Self::All(val) => domain::PaddingConfig::new(
                domain::PaddingOffset::new(val),
                domain::PaddingOffset::new(val),
                domain::PaddingOffset::new(val),
                domain::PaddingOffset::new(val),
            ),
            Self::Fields {
                top,
                bottom,
                left,
                right,
                horizontal,
                vertical,
            } => {
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
    pub fn into_domain(self) -> domain::BorderConfig {
        domain::BorderConfig::new(
            domain::BorderSize::new(self.size),
            self.color,
            domain::BorderRadius::new(self.radius),
        )
    }
}

fn default_border_color() -> DrawingColor {
    DrawingColor::Solid(crate::domain::shared::color::Color::new(0, 0, 0, 255))
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
    pub fn into_domain(self) -> domain::ModulesConfig {
        domain::ModulesConfig::new(
            self.left.into_iter().map(|m| m.into_domain()).collect(),
            self.center.into_iter().map(|m| m.into_domain()).collect(),
            self.right.into_iter().map(|m| m.into_domain()).collect(),
        )
    }
}

#[derive(Debug, Deserialize)]
pub struct ModuleConfigDto {
    name: String,
    enable: bool,
    #[serde(default)]
    engine: Option<String>,
    #[serde(flatten)]
    options: HashMap<String, serde_json::Value>,
}

impl ModuleConfigDto {
    pub fn into_domain(self) -> domain::ModuleConfig {
        let selection = match self.engine {
            Some(e) => domain::EngineSelection::Explicit(domain::EngineId::new(e)),
            None => domain::EngineSelection::Auto,
        };
        domain::ModuleConfig::new(self.name, self.enable, selection, self.options)
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
    pub fn into_domain(self) -> domain::RenderingMode {
        match self {
            RenderingModeDto::Immediate { fps_limit } => {
                domain::RenderingMode::new_immediate(fps_limit)
            }
            RenderingModeDto::Timebased { duration_ms } => {
                domain::RenderingMode::new_timebased(duration_ms)
            }
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
    },
}

impl Default for PartialMarginConfigDto {
    fn default() -> Self {
        Self::Fields {
            top: None,
            bottom: None,
            left: None,
            right: None,
            horizontal: None,
            vertical: None,
        }
    }
}

impl PartialMarginConfigDto {
    pub fn into_domain(self) -> domain::PartialMarginConfig {
        match self {
            Self::All(val) => domain::PartialMarginConfig::new(
                Some(domain::MarginOffset::new(val)),
                Some(domain::MarginOffset::new(val)),
                Some(domain::MarginOffset::new(val)),
                Some(domain::MarginOffset::new(val)),
            ),
            Self::Fields {
                top,
                bottom,
                left,
                right,
                horizontal,
                vertical,
            } => {
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
    },
}

impl Default for PartialPaddingConfigDto {
    fn default() -> Self {
        Self::Fields {
            top: None,
            bottom: None,
            left: None,
            right: None,
            horizontal: None,
            vertical: None,
        }
    }
}

impl PartialPaddingConfigDto {
    pub fn into_domain(self) -> domain::PartialPaddingConfig {
        match self {
            Self::All(val) => domain::PartialPaddingConfig::new(
                Some(domain::PaddingOffset::new(val)),
                Some(domain::PaddingOffset::new(val)),
                Some(domain::PaddingOffset::new(val)),
                Some(domain::PaddingOffset::new(val)),
            ),
            Self::Fields {
                top,
                bottom,
                left,
                right,
                horizontal,
                vertical,
            } => {
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
    pub fn into_domain(self) -> domain::PartialBorderConfig {
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
    module_gap: Option<u32>,
}

impl PartialBarConfigDto {
    pub fn into_domain(self) -> domain::PartialBarConfig {
        domain::PartialBarConfig::new(crate::domain::config::CreatePartialBarConfigCommand {
            background: self.background,
            height: self.height.map(crate::domain::shared::geometry::BarHeight::new),
            vertical_alignment: self.vertical_alignment.map(|v| v.into_domain()),
            border: self.border.map(|b| b.into_domain()),
            margin: self.margin.map(|m| m.into_domain()),
            padding: self.padding.map(|p| p.into_domain()),
            module_gap: self.module_gap.map(domain::ModuleGap::new),
            font_family: self.font_family.map(domain::FontFamily::new),
            font_size: self.font_size.map(domain::FontSize::new),
        })
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
    pub fn into_domain(self) -> domain::TooltipConfig {
        let default = domain::TooltipConfig::default();
        domain::TooltipConfig::new(
            self.background.and_then(|c| DrawingColor::parse(&c).ok()).unwrap_or_else(|| default.background().clone()),
            self.border_color.and_then(|c| DrawingColor::parse(&c).ok()).unwrap_or_else(|| default.border_color().clone()),
            self.text_color.and_then(|c| DrawingColor::parse(&c).ok()).unwrap_or_else(|| default.text_color().clone()),
            self.font.map(domain::FontFamily::new).or_else(|| default.font().cloned()),
            self.size.map(domain::FontSize::new).or_else(|| default.size()),
            self.radius.map(domain::BorderRadius::new).unwrap_or_else(|| default.radius()),
            self.border_width.map(domain::BorderSize::new).unwrap_or_else(|| default.border_width()),
            self.padding.map(domain::PaddingOffset::new).unwrap_or_else(|| default.padding()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // MockValidator removed

    #[test]
    fn test_margin_dto_all() {
        let dto = MarginConfigDto::All(10);
        let domain = dto.into_domain();
        assert_eq!(domain.top().value(), 10);
        assert_eq!(domain.bottom().value(), 10);
        assert_eq!(domain.left().value(), 10);
        assert_eq!(domain.right().value(), 10);
    }

    #[test]
    fn test_margin_dto_fields() {
        let dto = MarginConfigDto::Fields {
            top: Some(5),
            bottom: None,
            left: None,
            right: None,
            horizontal: Some(10),
            vertical: Some(20),
        };
        let domain = dto.into_domain();
        assert_eq!(domain.top().value(), 5);
        assert_eq!(domain.bottom().value(), 20);
        assert_eq!(domain.left().value(), 10);
        assert_eq!(domain.right().value(), 10);
    }

    #[test]
    fn test_padding_dto_fields() {
        let dto = PaddingConfigDto::Fields {
            top: Some(5),
            bottom: None,
            left: None,
            right: None,
            horizontal: Some(10),
            vertical: Some(20),
        };
        let domain = dto.into_domain();
        assert_eq!(domain.top().value(), 5);
        assert_eq!(domain.bottom().value(), 20);
        assert_eq!(domain.left().value(), 10);
        assert_eq!(domain.right().value(), 10);
    }

    #[test]
    fn test_rendering_mode_dto() {
        let mode1 = RenderingModeDto::Immediate { fps_limit: Some(60) };
        if let domain::RenderingMode::Immediate { fps_limit } = mode1.into_domain() {
            assert_eq!(fps_limit, Some(60));
        } else {
            panic!("Expected Immediate");
        }

        let mode2 = RenderingModeDto::Timebased { duration_ms: 50 };
        if let domain::RenderingMode::Timebased { duration_ms } = mode2.into_domain() {
            assert_eq!(duration_ms, 50);
        } else {
            panic!("Expected Timebased");
        }
    }
}
