use crate::utils::ParsedColor;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("IO error reading config: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum ReloadError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Registry(#[from] crate::modules::RegistryError),
}

pub type Result<T> = std::result::Result<T, ConfigError>;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    bar: BarConfig,
    modules: ModulesConfig,
    #[serde(default)]
    rendering: RenderingMode,
}

impl Config {
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_str(&content)
    }

    pub fn from_str(content: &str) -> Result<Self> {
        Ok(toml::from_str(content)?)
    }

    pub fn bar(&self) -> &BarConfig {
        &self.bar
    }

    pub fn modules(&self) -> &ModulesConfig {
        &self.modules
    }

    pub fn rendering(&self) -> &RenderingMode {
        &self.rendering
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum RenderingMode {
    Immediate {
        #[serde(default)]
        fps_limit: Option<u32>,
    },
    Timebased {
        #[serde(default = "default_timebased_duration_ms")]
        duration_ms: u64,
    },
}

impl Default for RenderingMode {
    fn default() -> Self {
        Self::Timebased {
            duration_ms: default_timebased_duration_ms(),
        }
    }
}

impl RenderingMode {
    #[cfg(test)]
    pub fn fps_limit(&self) -> Option<u32> {
        match self {
            RenderingMode::Immediate { fps_limit } => *fps_limit,
            RenderingMode::Timebased { .. } => None,
        }
    }

    #[cfg(test)]
    pub fn duration_ms(&self) -> Option<u64> {
        match self {
            RenderingMode::Immediate { .. } => None,
            RenderingMode::Timebased { duration_ms } => Some(*duration_ms),
        }
    }
}

fn default_timebased_duration_ms() -> u64 {
    100
}

#[derive(Debug, Default, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum VerticalAlignment {
    Top,
    #[default]
    Center,
    Bottom,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BarConfig {
    #[serde(default = "default_background")]
    background: ParsedColor,
    #[serde(default = "default_height")]
    height: u32,
    #[serde(default)]
    vertical_alignment: VerticalAlignment,
    #[serde(default)]
    border: BorderConfig,
    #[serde(default)]
    margin: MarginConfig,
    #[serde(default)]
    unfocused: Option<PartialBarConfig>,
}

#[derive(Debug, Deserialize, Clone, Default, PartialEq)]
pub struct PartialMarginConfig {
    pub top: Option<i32>,
    pub bottom: Option<i32>,
    pub left: Option<i32>,
    pub right: Option<i32>,
}

#[derive(Debug, Deserialize, Clone, Default, PartialEq)]
pub struct PartialBorderConfig {
    pub size: Option<f32>,
    pub color: Option<ParsedColor>,
    pub radius: Option<f32>,
}

#[derive(Debug, Deserialize, Clone, Default, PartialEq)]
pub struct PartialBarConfig {
    pub background: Option<ParsedColor>,
    pub height: Option<u32>,
    pub vertical_alignment: Option<VerticalAlignment>,
    pub border: Option<PartialBorderConfig>,
    pub margin: Option<PartialMarginConfig>,
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            background: default_background(),
            height: default_height(),
            vertical_alignment: VerticalAlignment::default(),
            border: BorderConfig::default(),
            margin: MarginConfig::default(),
            unfocused: None,
        }
    }
}

fn default_background() -> ParsedColor {
    ParsedColor::Solid(tiny_skia::Color::BLACK)
}

fn default_height() -> u32 {
    30
}

impl BarConfig {
    pub fn background(&self) -> &ParsedColor {
        &self.background
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn vertical_alignment(&self) -> VerticalAlignment {
        self.vertical_alignment
    }

    pub fn border(&self) -> &BorderConfig {
        &self.border
    }

    pub fn margin(&self) -> &MarginConfig {
        &self.margin
    }

    pub fn as_unfocused(&self) -> BarConfig {
        let mut base = self.clone();
        if let Some(unfocused) = &self.unfocused {
            if let Some(bg) = &unfocused.background {
                base.background = bg.clone();
            }
            if let Some(h) = unfocused.height {
                base.height = h;
            }
            if let Some(va) = unfocused.vertical_alignment {
                base.vertical_alignment = va;
            }
            if let Some(pb) = &unfocused.border {
                if let Some(s) = pb.size {
                    base.border.size = s;
                }
                if let Some(c) = &pb.color {
                    base.border.color = c.clone();
                }
                if let Some(r) = pb.radius {
                    base.border.radius = r;
                }
            }
            if let Some(pm) = &unfocused.margin {
                if let Some(t) = pm.top {
                    base.margin.top = t;
                }
                if let Some(b) = pm.bottom {
                    base.margin.bottom = b;
                }
                if let Some(l) = pm.left {
                    base.margin.left = l;
                }
                if let Some(r) = pm.right {
                    base.margin.right = r;
                }
            }
        }
        base
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct MarginConfig {
    #[serde(default)]
    top: i32,
    #[serde(default)]
    bottom: i32,
    #[serde(default)]
    left: i32,
    #[serde(default)]
    right: i32,
}

impl MarginConfig {
    pub fn top(&self) -> i32 {
        self.top
    }

    pub fn bottom(&self) -> i32 {
        self.bottom
    }

    pub fn left(&self) -> i32 {
        self.left
    }

    pub fn right(&self) -> i32 {
        self.right
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct BorderConfig {
    #[serde(default)]
    size: f32,
    #[serde(default = "default_border_color")]
    color: ParsedColor,
    #[serde(default)]
    radius: f32,
}

impl Default for BorderConfig {
    fn default() -> Self {
        Self {
            size: 0.0,
            color: default_border_color(),
            radius: 0.0,
        }
    }
}

fn default_border_color() -> ParsedColor {
    ParsedColor::Solid(tiny_skia::Color::BLACK)
}

impl BorderConfig {
    pub fn size(&self) -> f32 {
        self.size
    }

    pub fn color(&self) -> &ParsedColor {
        &self.color
    }

    pub fn radius(&self) -> f32 {
        self.radius
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ModulesConfig {
    #[serde(default)]
    left: Vec<ModuleConfig>,
    #[serde(default)]
    center: Vec<ModuleConfig>,
    #[serde(default)]
    right: Vec<ModuleConfig>,
}

impl ModulesConfig {
    pub fn left(&self) -> &[ModuleConfig] {
        &self.left
    }

    pub fn center(&self) -> &[ModuleConfig] {
        &self.center
    }

    pub fn right(&self) -> &[ModuleConfig] {
        &self.right
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ModuleConfig {
    name: String,
    enable: bool,
    #[serde(flatten)]
    options: HashMap<String, serde_json::Value>,
}

impl ModuleConfig {
    #[cfg(test)]
    pub fn new(name: String, enable: bool, options: HashMap<String, serde_json::Value>) -> Self {
        Self {
            name,
            enable,
            options,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_enabled(&self) -> bool {
        self.enable
    }

    pub fn options(&self) -> &HashMap<String, serde_json::Value> {
        &self.options
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_str() {
        let toml_str = r##"
            [bar]
            background = "#1a1b26"
            height = 30
            [bar.border]
            radius = 8.0
            color = "#7aa2f7"
            size = 1.0
            [bar.margin]
            top = 5
            bottom = 0
            left = 10
            right = 10

            [modules]
            left = [{ name = "workspace", enable = true }]
            center = [{ name = "hour", enable = true }]
            right = [{ name = "network", enable = true }]

            [rendering]
            mode = "immediate"
            fps_limit = 60
        "##;

        let config = Config::from_str(toml_str).unwrap();
        assert_eq!(
            config.bar().background(),
            &ParsedColor::try_from("#1a1b26").unwrap()
        );
        assert_eq!(config.bar().height(), 30);
        assert_eq!(config.bar().vertical_alignment(), VerticalAlignment::Center);

        let border = config.bar().border();
        assert_eq!(border.radius(), 8.0);
        assert_eq!(border.color(), &ParsedColor::try_from("#7aa2f7").unwrap());

        let margin = config.bar().margin();
        assert_eq!(margin.top(), 5);
        assert_eq!(margin.bottom(), 0);
        assert_eq!(margin.left(), 10);
        assert_eq!(margin.right(), 10);

        assert_eq!(config.modules().left().len(), 1);
        assert_eq!(config.modules().left()[0].name(), "workspace");
        assert!(config.modules().left()[0].is_enabled());

        assert_eq!(config.modules().center().len(), 1);
        assert_eq!(config.modules().right().len(), 1);
        assert_eq!(
            config.rendering(),
            &RenderingMode::Immediate {
                fps_limit: Some(60),
            }
        );
    }

    #[test]
    fn test_config_getters() {
        let toml_str = r##"
            [bar]
            background = "#000000"
            height = 30
            [modules]
        "##;
        let config = Config::from_str(toml_str).unwrap();

        let modules = config.modules();
        assert!(modules.left().is_empty());
        assert!(modules.center().is_empty());
        assert!(modules.right().is_empty());

        let bar = config.bar();
        let border = bar.border();
        assert_eq!(border.size(), 0.0);

        let margin = bar.margin();
        assert_eq!(margin.left(), 0);
        assert_eq!(margin.right(), 0);
        assert_eq!(margin.bottom(), 0);

        assert_eq!(
            config.rendering(),
            &RenderingMode::Timebased { duration_ms: 100 }
        );
    }

    #[test]
    fn test_default_configs() {
        let bar_default = BarConfig::default();
        assert_eq!(
            bar_default.background(),
            &ParsedColor::Solid(tiny_skia::Color::BLACK)
        );
        assert_eq!(bar_default.height(), 30);

        let margin_default = MarginConfig::default();
        assert_eq!(margin_default.top(), 0);

        let border_default = BorderConfig::default();
        assert_eq!(border_default.size(), 0.0);
        assert_eq!(
            border_default.color(),
            &ParsedColor::Solid(tiny_skia::Color::BLACK)
        );
    }

    #[test]
    fn test_module_config_new() {
        let mut options = HashMap::new();
        options.insert(
            "key".to_string(),
            serde_json::Value::String("value".to_string()),
        );
        let mc = ModuleConfig::new("test".to_string(), true, options);
        assert_eq!(mc.name(), "test");
        assert!(mc.is_enabled());
        assert_eq!(mc.options().get("key").unwrap(), "value");
    }

    #[test]
    fn test_bar_config_as_unfocused() {
        let toml_str = r##"
            [bar]
            background = "#000000"
            height = 30
            [bar.unfocused]
            background = "#ffffff"
            height = 20
            [bar.unfocused.border]
            size = 2.0
            [modules]
        "##;
        let config = Config::from_str(toml_str).unwrap();
        let bar = config.bar();
        let unfocused = bar.as_unfocused();

        assert_eq!(bar.background(), &ParsedColor::try_from("#000000").unwrap());
        assert_eq!(bar.height(), 30);

        assert_eq!(
            unfocused.background(),
            &ParsedColor::try_from("#ffffff").unwrap()
        );
        assert_eq!(unfocused.height(), 20);
        assert_eq!(unfocused.border().size(), 2.0);
    }

    #[test]
    fn test_rendering_mode_immediate_without_limit() {
        let toml_str = r##"
            [bar]
            [modules]
            [rendering]
            mode = "immediate"
        "##;
        let config = Config::from_str(toml_str).unwrap();

        assert_eq!(config.rendering().fps_limit(), None);
        assert_eq!(config.rendering().duration_ms(), None);
    }

    #[test]
    fn test_rendering_mode_timebased_custom_duration() {
        let toml_str = r##"
            [bar]
            [modules]
            [rendering]
            mode = "timebased"
            duration_ms = 250
        "##;
        let config = Config::from_str(toml_str).unwrap();

        assert_eq!(config.rendering().fps_limit(), None);
        assert_eq!(config.rendering().duration_ms(), Some(250));
    }

    #[test]
    fn test_bar_config_defaults() {
        let bar = BarConfig::default();
        assert_eq!(bar.height(), 30);
        assert_eq!(bar.vertical_alignment(), VerticalAlignment::Center);
    }

    #[test]
    fn test_partial_configs() {
        let partial_margin = PartialMarginConfig {
            top: Some(5),
            bottom: Some(10),
            ..Default::default()
        };
        assert_eq!(partial_margin.top, Some(5));
        
        let def_margin = PartialMarginConfig::default();
        assert!(def_margin.top.is_none());
        assert_eq!(def_margin, PartialMarginConfig::default());
        
        let partial_bar = PartialBarConfig {
            height: Some(40),
            ..Default::default()
        };
        assert_eq!(partial_bar.height, Some(40));
        assert_eq!(partial_bar, partial_bar.clone());
        
        let partial_border = PartialBorderConfig {
            size: Some(1.0),
            ..Default::default()
        };
        assert_eq!(partial_border.size, Some(1.0));
        assert_eq!(partial_border, partial_border.clone());
    }

    #[test]
    fn test_config_error_parse() {
        let err = Config::from_str("invalid = ").unwrap_err();
        assert!(format!("{}", err).contains("parse"));
    }

    #[test]
    fn test_config_error_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let err = ConfigError::Io(io_err);
        assert!(format!("{}", err).contains("IO error"));
    }
}
