use crate::domain::color::DrawingColor;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct FontFamily(String);

impl FontFamily {
    pub fn new(family: String) -> Self {
        Self(family)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontSize(f32);

impl FontSize {
    pub fn new(size: f32) -> Self {
        Self(size)
    }

    pub fn value(&self) -> f32 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    bar: BarConfig,
    modules: ModulesConfig,
    rendering: RenderingMode,
}

impl Config {
    pub fn new(bar: BarConfig, modules: ModulesConfig, rendering: RenderingMode) -> Self {
        Self {
            bar,
            modules,
            rendering,
        }
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

impl Default for Config {
    fn default() -> Self {
        Self {
            bar: BarConfig::default(),
            modules: ModulesConfig::default(),
            rendering: RenderingMode::default(),
        }
    }
}

impl Default for RenderingMode {
    fn default() -> Self {
        Self::Timebased {
            duration_ms: 100,
        }
    }
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            background: DrawingColor::Solid(crate::domain::color::Color::new(0, 0, 0, 255)),
            height: 30,
            vertical_alignment: VerticalAlignment::default(),
            border: BorderConfig::default(),
            margin: MarginConfig::default(),
            font_family: FontFamily::new("".to_string()),
            font_size: FontSize::new(14.0),
            unfocused: None,
        }
    }
}

impl Default for BorderConfig {
    fn default() -> Self {
        Self {
            size: 0.0,
            color: DrawingColor::Solid(crate::domain::color::Color::new(0, 0, 0, 255)),
            radius: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderingMode {
    Immediate { fps_limit: Option<u32> },
    Timebased { duration_ms: u64 },
}

impl RenderingMode {
    pub fn new_immediate(fps_limit: Option<u32>) -> Self {
        Self::Immediate { fps_limit }
    }

    pub fn new_timebased(duration_ms: u64) -> Self {
        Self::Timebased { duration_ms }
    }

    pub fn fps_limit(&self) -> Option<u32> {
        match self {
            RenderingMode::Immediate { fps_limit } => *fps_limit,
            RenderingMode::Timebased { .. } => None,
        }
    }

    pub fn duration_ms(&self) -> Option<u64> {
        match self {
            RenderingMode::Immediate { .. } => None,
            RenderingMode::Timebased { duration_ms } => Some(*duration_ms),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerticalAlignment {
    Top,
    #[default]
    Center,
    Bottom,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BarConfig {
    background: DrawingColor,
    height: u32,
    vertical_alignment: VerticalAlignment,
    border: BorderConfig,
    margin: MarginConfig,
    font_family: FontFamily,
    font_size: FontSize,
    unfocused: Option<PartialBarConfig>,
}

impl BarConfig {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        background: DrawingColor,
        height: u32,
        vertical_alignment: VerticalAlignment,
        border: BorderConfig,
        margin: MarginConfig,
        font_family: FontFamily,
        font_size: FontSize,
        unfocused: Option<PartialBarConfig>,
    ) -> Self {
        Self {
            background,
            height,
            vertical_alignment,
            border,
            margin,
            font_family,
            font_size,
            unfocused,
        }
    }

    pub fn background(&self) -> &DrawingColor {
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

    pub fn font_family(&self) -> &FontFamily {
        &self.font_family
    }

    pub fn font_size(&self) -> FontSize {
        self.font_size
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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MarginConfig {
    top: i32,
    bottom: i32,
    left: i32,
    right: i32,
}

impl MarginConfig {
    pub fn new(top: i32, bottom: i32, left: i32, right: i32) -> Self {
        Self {
            top,
            bottom,
            left,
            right,
        }
    }

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

#[derive(Debug, Clone, PartialEq)]
pub struct BorderConfig {
    size: f32,
    color: DrawingColor,
    radius: f32,
}

impl BorderConfig {
    pub fn new(size: f32, color: DrawingColor, radius: f32) -> Self {
        Self {
            size,
            color,
            radius,
        }
    }

    pub fn size(&self) -> f32 {
        self.size
    }

    pub fn color(&self) -> &DrawingColor {
        &self.color
    }

    pub fn radius(&self) -> f32 {
        self.radius
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModulesConfig {
    left: Vec<ModuleConfig>,
    center: Vec<ModuleConfig>,
    right: Vec<ModuleConfig>,
}

impl ModulesConfig {
    pub fn new(
        left: Vec<ModuleConfig>,
        center: Vec<ModuleConfig>,
        right: Vec<ModuleConfig>,
    ) -> Self {
        Self {
            left,
            center,
            right,
        }
    }

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

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleConfig {
    name: String,
    enable: bool,
    options: HashMap<String, serde_json::Value>,
}

impl ModuleConfig {
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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PartialMarginConfig {
    pub top: Option<i32>,
    pub bottom: Option<i32>,
    pub left: Option<i32>,
    pub right: Option<i32>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PartialBorderConfig {
    pub size: Option<f32>,
    pub color: Option<DrawingColor>,
    pub radius: Option<f32>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PartialBarConfig {
    pub background: Option<DrawingColor>,
    pub height: Option<u32>,
    pub vertical_alignment: Option<VerticalAlignment>,
    pub border: Option<PartialBorderConfig>,
    pub margin: Option<PartialMarginConfig>,
}
