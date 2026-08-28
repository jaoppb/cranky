use crate::shared::config::domain;
use crate::shared::primitives::color::DrawingColor;
use crate::shared::rendering::ports::font::FontValidatorPort;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Default)]
pub struct ConfigDto {
    #[serde(default)]
    root: RootConfigDto,
    #[serde(default)]
    modules: ModulesConfigDto,
    #[serde(default)]
    rendering: RenderingModeDto,
    #[serde(default)]
    metrics: crate::features::metrics::domain::MetricsConfig,
    #[serde(default)]
    tooltip: TooltipConfigDto,
}

impl ConfigDto {
    #[must_use]
    pub fn into_domain<V: FontValidatorPort>(self, _validator: &V) -> domain::Config {
        let root = self.root.into_domain();
        let modules = self.modules.into_domain();
        let rendering = self.rendering.into_domain();
        let tooltip = self.tooltip.into_domain();

        domain::Config::new(root, modules, rendering, self.metrics, tooltip)
    }
}

#[derive(Debug, Deserialize)]
pub struct RootConfigDto {
    #[serde(default = "default_root_name")]
    name: String,
    #[serde(default = "default_height")]
    height: u32,
    #[serde(default)]
    vertical_alignment: VerticalAlignmentDto,
    #[serde(default)]
    margin: MarginConfigDto,
    #[serde(default)]
    unfocused: Option<PartialRootConfigDto>,
    #[serde(flatten)]
    options: HashMap<String, serde_json::Value>,
}

fn default_root_name() -> String {
    "bar".to_string()
}

const fn default_height() -> u32 {
    30
}

impl Default for RootConfigDto {
    fn default() -> Self {
        Self {
            name: default_root_name(),
            height: default_height(),
            vertical_alignment: VerticalAlignmentDto::default(),
            margin: MarginConfigDto::default(),
            unfocused: None,
            options: HashMap::new(),
        }
    }
}

fn json_value_to_dynamic(v: serde_json::Value) -> crate::shared::primitives::DynamicValue {
    match v {
        serde_json::Value::Null => crate::shared::primitives::DynamicValue::Null,
        serde_json::Value::Bool(b) => crate::shared::primitives::DynamicValue::Bool(b),
        serde_json::Value::Number(n) => {
            crate::shared::primitives::DynamicValue::Number(n.as_f64().unwrap_or(0.0))
        }
        serde_json::Value::String(s) => crate::shared::primitives::DynamicValue::String(s),
        serde_json::Value::Array(arr) => crate::shared::primitives::DynamicValue::Array(
            arr.into_iter().map(json_value_to_dynamic).collect(),
        ),
        serde_json::Value::Object(map) => crate::shared::primitives::DynamicValue::Map(
            map.into_iter()
                .map(|(k, v)| (k, json_value_to_dynamic(v)))
                .collect(),
        ),
    }
}

fn json_map_to_options(
    map: HashMap<String, serde_json::Value>,
) -> crate::shared::primitives::ModuleOptions {
    crate::shared::primitives::ModuleOptions::new(
        map.into_iter()
            .map(|(k, v)| (k, json_value_to_dynamic(v)))
            .collect(),
    )
}

impl RootConfigDto {
    #[must_use]
    pub fn into_domain(self) -> domain::RootConfig {
        domain::RootConfig::new(crate::shared::config::domain::CreateRootConfigCommand::new(
            crate::shared::primitives::ModuleName::new(self.name),
            crate::shared::primitives::geometry::BarHeight::new(self.height),
            self.vertical_alignment.into_domain(),
            self.margin.into_domain(),
            self.unfocused.map(PartialRootConfigDto::into_domain),
            json_map_to_options(self.options),
        ))
    }
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
    #[must_use]
    pub const fn into_domain(self) -> domain::VerticalAlignment {
        match self {
            Self::Top => domain::VerticalAlignment::Top,
            Self::Center => domain::VerticalAlignment::Center,
            Self::Bottom => domain::VerticalAlignment::Bottom,
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
    #[must_use]
    pub const fn into_domain(self) -> domain::MarginConfig {
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
                let t = match top {
                    Some(v) => v,
                    None => match vertical {
                        Some(v) => v,
                        None => 0,
                    },
                };
                let b = match bottom {
                    Some(v) => v,
                    None => match vertical {
                        Some(v) => v,
                        None => 0,
                    },
                };
                let l = match left {
                    Some(v) => v,
                    None => match horizontal {
                        Some(v) => v,
                        None => 0,
                    },
                };
                let r = match right {
                    Some(v) => v,
                    None => match horizontal {
                        Some(v) => v,
                        None => 0,
                    },
                };
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

#[derive(Debug, Deserialize, Default)]
pub struct ModulesConfigDto {
    #[serde(flatten)]
    modules: HashMap<String, ModuleEntryDto>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ModuleEntryDto {
    Detailed {
        #[serde(default = "default_true")]
        enable: bool,
        #[serde(default)]
        engine: Option<String>,
        #[serde(flatten)]
        options: HashMap<String, serde_json::Value>,
    },
    OptionsOnly(HashMap<String, serde_json::Value>),
}

const fn default_true() -> bool {
    true
}

impl ModulesConfigDto {
    #[must_use]
    pub fn into_domain(self) -> domain::ModulesConfig {
        let mut map = HashMap::new();
        for (name_str, entry) in self.modules {
            let mod_name = crate::shared::primitives::ModuleName::new(name_str);
            let mod_cfg = match entry {
                ModuleEntryDto::Detailed {
                    enable,
                    engine,
                    options,
                } => {
                    let selection = engine.map_or(domain::EngineSelection::Auto, |e| {
                        domain::EngineSelection::Explicit(domain::EngineId::new(e))
                    });
                    domain::ModuleConfig::new(
                        mod_name.clone(),
                        enable,
                        selection,
                        json_map_to_options(options),
                    )
                }
                ModuleEntryDto::OptionsOnly(options) => domain::ModuleConfig::new(
                    mod_name.clone(),
                    true,
                    domain::EngineSelection::Auto,
                    json_map_to_options(options),
                ),
            };
            map.insert(mod_name, mod_cfg);
        }
        domain::ModulesConfig::new(map)
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
    #[must_use]
    pub const fn into_domain(self) -> domain::RenderingMode {
        match self {
            Self::Immediate { fps_limit } => domain::RenderingMode::new_immediate(fps_limit),
            Self::Timebased { duration_ms } => domain::RenderingMode::new_timebased(duration_ms),
        }
    }
}

const fn default_timebased_duration_ms() -> u64 {
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
    #[must_use]
    pub const fn into_domain(self) -> domain::PartialMarginConfig {
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
                let t = match top {
                    Some(v) => Some(domain::MarginOffset::new(v)),
                    None => match vertical {
                        Some(v) => Some(domain::MarginOffset::new(v)),
                        None => None,
                    },
                };
                let b = match bottom {
                    Some(v) => Some(domain::MarginOffset::new(v)),
                    None => match vertical {
                        Some(v) => Some(domain::MarginOffset::new(v)),
                        None => None,
                    },
                };
                let l = match left {
                    Some(v) => Some(domain::MarginOffset::new(v)),
                    None => match horizontal {
                        Some(v) => Some(domain::MarginOffset::new(v)),
                        None => None,
                    },
                };
                let r = match right {
                    Some(v) => Some(domain::MarginOffset::new(v)),
                    None => match horizontal {
                        Some(v) => Some(domain::MarginOffset::new(v)),
                        None => None,
                    },
                };
                domain::PartialMarginConfig::new(t, b, l, r)
            }
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct PartialRootConfigDto {
    #[serde(default)]
    height: Option<u32>,
    #[serde(default)]
    vertical_alignment: Option<VerticalAlignmentDto>,
    #[serde(default)]
    margin: Option<PartialMarginConfigDto>,
}

impl PartialRootConfigDto {
    #[must_use]
    pub fn into_domain(self) -> domain::PartialRootConfig {
        domain::PartialRootConfig::new(
            crate::shared::config::domain::CreatePartialRootConfigCommand::new(
                self.height
                    .map(crate::shared::primitives::geometry::BarHeight::new),
                self.vertical_alignment
                    .map(VerticalAlignmentDto::into_domain),
                self.margin.map(PartialMarginConfigDto::into_domain),
            ),
        )
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
                .map(domain::FontFamily::new)
                .or_else(|| default.font().cloned()),
            self.size
                .map(domain::FontSize::new)
                .or_else(|| default.size()),
            self.radius
                .map_or_else(|| default.radius(), domain::BorderRadius::new),
            self.border_width
                .map_or_else(|| default.border_width(), domain::BorderSize::new),
            self.padding
                .map_or_else(|| default.padding(), domain::PaddingOffset::new),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_rendering_mode_dto() {
        let mode1 = RenderingModeDto::Immediate {
            fps_limit: Some(60),
        };
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

    #[test]
    fn test_root_config_dto() {
        let toml_str = r#"
            name = "bar"
            height = 36
            margin = 5
            [options]
            foo = "bar"
        "#;
        let dto: RootConfigDto = toml::from_str(toml_str).unwrap();
        let domain = dto.into_domain();
        assert_eq!(domain.name().as_str(), "bar");
        assert_eq!(domain.height().value(), 36);
        assert_eq!(domain.margin().top().value(), 5);
    }

    #[test]
    fn test_modules_config_dto() {
        let toml_str = r#"
            [hour]
            format = "%H:%M:%S"

            [workspace]
            enable = true
            engine = "rhai"
            border_radius = 4.0
        "#;
        let dto: ModulesConfigDto = toml::from_str(toml_str).unwrap();
        let domain = dto.into_domain();
        let hour = domain
            .get(&crate::shared::primitives::ModuleName::new("hour"))
            .unwrap();
        assert_eq!(
            hour.options().get("format").and_then(|v| v.as_str()),
            Some("%H:%M:%S")
        );

        let ws = domain
            .get(&crate::shared::primitives::ModuleName::new("workspace"))
            .unwrap();
        assert!(ws.is_enabled());
        assert_eq!(ws.engine().as_explicit().unwrap().as_str(), "rhai");
    }
}
