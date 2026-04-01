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

pub type Result<T> = std::result::Result<T, ConfigError>;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    bar: BarConfig,
    modules: ModulesConfig,
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
    background: String,
    #[serde(default = "default_height")]
    height: u32,
    #[serde(default)]
    vertical_alignment: VerticalAlignment,
    #[serde(default)]
    border: BorderConfig,
    #[serde(default)]
    margin: MarginConfig,
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            background: default_background(),
            height: default_height(),
            vertical_alignment: VerticalAlignment::default(),
            border: BorderConfig::default(),
            margin: MarginConfig::default(),
        }
    }
}

fn default_background() -> String {
    "#000000".to_string()
}

fn default_height() -> u32 {
    30
}

impl BarConfig {
    pub fn background(&self) -> &str {
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
    color: String,
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

fn default_border_color() -> String {
    "#000000".to_string()
}

impl BorderConfig {
    pub fn size(&self) -> f32 {
        self.size
    }

    pub fn color(&self) -> &str {
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
        "##;

        let config = Config::from_str(toml_str).unwrap();
        assert_eq!(config.bar().background(), "#1a1b26");
        assert_eq!(config.bar().height(), 30);
        assert_eq!(config.bar().vertical_alignment(), VerticalAlignment::Center);
        
        let border = config.bar().border();
        assert_eq!(border.radius(), 8.0);
        assert_eq!(border.color(), "#7aa2f7");
        
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
    }

    #[test]
    fn test_default_configs() {
        let bar_default = BarConfig::default();
        assert_eq!(bar_default.background(), "#000000");
        assert_eq!(bar_default.height(), 30);
        
        let margin_default = MarginConfig::default();
        assert_eq!(margin_default.top(), 0);
        
        let border_default = BorderConfig::default();
        assert_eq!(border_default.size(), 0.0);
        assert_eq!(border_default.color(), "#000000");
    }

    #[test]
    fn test_module_config_new() {
        let mut options = HashMap::new();
        options.insert("key".to_string(), serde_json::Value::String("value".to_string()));
        let mc = ModuleConfig::new("test".to_string(), true, options);
        assert_eq!(mc.name(), "test");
        assert!(mc.is_enabled());
        assert_eq!(mc.options().get("key").unwrap(), "value");
    }
}
