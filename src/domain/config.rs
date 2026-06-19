use crate::domain::shared::color::DrawingColor;
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BorderSize(f32);

impl BorderSize {
    pub fn new(size: f32) -> Self {
        Self(size)
    }

    pub fn value(&self) -> f32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BorderRadius(f32);

impl BorderRadius {
    pub fn new(radius: f32) -> Self {
        Self(radius)
    }

    pub fn value(&self) -> f32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MarginOffset(i32);

impl MarginOffset {
    pub fn new(offset: i32) -> Self {
        Self(offset)
    }

    pub fn value(&self) -> i32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PaddingOffset(u32);

impl PaddingOffset {
    pub fn new(offset: u32) -> Self {
        Self(offset)
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
#[derive(Default)]
pub struct Config {
    bar: BarConfig,
    modules: ModulesConfig,
    rendering: RenderingMode,
    metrics: crate::domain::metrics::MetricsConfig,
}

impl Config {
    pub fn new(bar: BarConfig, modules: ModulesConfig, rendering: RenderingMode, metrics: crate::domain::metrics::MetricsConfig) -> Self {
        Self {
            bar,
            modules,
            rendering,
            metrics,
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

    pub fn metrics(&self) -> &crate::domain::metrics::MetricsConfig {
        &self.metrics
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
            background: DrawingColor::Solid(crate::domain::shared::color::Color::new(0, 0, 0, 255)),
            height: 30,
            vertical_alignment: VerticalAlignment::default(),
            border: BorderConfig::default(),
            margin: MarginConfig::default(),
            padding: PaddingConfig::default(),
            font_family: FontFamily::new("".to_string()),
            font_size: FontSize::new(14.0),
            unfocused: None,
        }
    }
}

impl Default for BorderConfig {
    fn default() -> Self {
        Self {
            size: BorderSize::new(0.0),
            color: DrawingColor::Solid(crate::domain::shared::color::Color::new(0, 0, 0, 255)),
            radius: BorderRadius::new(0.0),
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
    padding: PaddingConfig,
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
        padding: PaddingConfig,
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
            padding,
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

    pub fn padding(&self) -> &PaddingConfig {
        &self.padding
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
            if let Some(bg) = unfocused.background() {
                base.background = bg.clone();
            }
            if let Some(h) = unfocused.height() {
                base.height = h;
            }
            if let Some(va) = unfocused.vertical_alignment() {
                base.vertical_alignment = va;
            }
            if let Some(pb) = unfocused.border() {
                if let Some(s) = pb.size() {
                    base.border.size = s;
                }
                if let Some(c) = pb.color() {
                    base.border.color = c.clone();
                }
                if let Some(r) = pb.radius() {
                    base.border.radius = r;
                }
            }
            if let Some(pm) = unfocused.margin() {
                if let Some(t) = pm.top() {
                    base.margin.top = t;
                }
                if let Some(b) = pm.bottom() {
                    base.margin.bottom = b;
                }
                if let Some(l) = pm.left() {
                    base.margin.left = l;
                }
                if let Some(r) = pm.right() {
                    base.margin.right = r;
                }
            }
            if let Some(pp) = unfocused.padding() {
                if let Some(t) = pp.top() {
                    base.padding.top = t;
                }
                if let Some(b) = pp.bottom() {
                    base.padding.bottom = b;
                }
                if let Some(l) = pp.left() {
                    base.padding.left = l;
                }
                if let Some(r) = pp.right() {
                    base.padding.right = r;
                }
            }
            if let Some(ff) = unfocused.font_family() {
                base.font_family = ff.clone();
            }
            if let Some(fs) = unfocused.font_size() {
                base.font_size = fs;
            }
        }
        base
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MarginConfig {
    top: MarginOffset,
    bottom: MarginOffset,
    left: MarginOffset,
    right: MarginOffset,
}

impl MarginConfig {
    pub fn new(top: MarginOffset, bottom: MarginOffset, left: MarginOffset, right: MarginOffset) -> Self {
        Self {
            top,
            bottom,
            left,
            right,
        }
    }

    pub fn top(&self) -> MarginOffset {
        self.top
    }

    pub fn bottom(&self) -> MarginOffset {
        self.bottom
    }

    pub fn left(&self) -> MarginOffset {
        self.left
    }

    pub fn right(&self) -> MarginOffset {
        self.right
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PaddingConfig {
    top: PaddingOffset,
    bottom: PaddingOffset,
    left: PaddingOffset,
    right: PaddingOffset,
}

impl PaddingConfig {
    pub fn new(top: PaddingOffset, bottom: PaddingOffset, left: PaddingOffset, right: PaddingOffset) -> Self {
        Self {
            top,
            bottom,
            left,
            right,
        }
    }

    pub fn top(&self) -> PaddingOffset {
        self.top
    }

    pub fn bottom(&self) -> PaddingOffset {
        self.bottom
    }

    pub fn left(&self) -> PaddingOffset {
        self.left
    }

    pub fn right(&self) -> PaddingOffset {
        self.right
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BorderConfig {
    size: BorderSize,
    color: DrawingColor,
    radius: BorderRadius,
}

impl BorderConfig {
    pub fn new(size: BorderSize, color: DrawingColor, radius: BorderRadius) -> Self {
        Self {
            size,
            color,
            radius,
        }
    }

    pub fn size(&self) -> BorderSize {
        self.size
    }

    pub fn color(&self) -> &DrawingColor {
        &self.color
    }

    pub fn radius(&self) -> BorderRadius {
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
    top: Option<MarginOffset>,
    bottom: Option<MarginOffset>,
    left: Option<MarginOffset>,
    right: Option<MarginOffset>,
}

impl PartialMarginConfig {
    pub fn new(
        top: Option<MarginOffset>,
        bottom: Option<MarginOffset>,
        left: Option<MarginOffset>,
        right: Option<MarginOffset>,
    ) -> Self {
        Self {
            top,
            bottom,
            left,
            right,
        }
    }

    pub fn top(&self) -> Option<MarginOffset> {
        self.top
    }
    pub fn bottom(&self) -> Option<MarginOffset> {
        self.bottom
    }
    pub fn left(&self) -> Option<MarginOffset> {
        self.left
    }
    pub fn right(&self) -> Option<MarginOffset> {
        self.right
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PartialPaddingConfig {
    top: Option<PaddingOffset>,
    bottom: Option<PaddingOffset>,
    left: Option<PaddingOffset>,
    right: Option<PaddingOffset>,
}

impl PartialPaddingConfig {
    pub fn new(
        top: Option<PaddingOffset>,
        bottom: Option<PaddingOffset>,
        left: Option<PaddingOffset>,
        right: Option<PaddingOffset>,
    ) -> Self {
        Self {
            top,
            bottom,
            left,
            right,
        }
    }

    pub fn top(&self) -> Option<PaddingOffset> {
        self.top
    }
    pub fn bottom(&self) -> Option<PaddingOffset> {
        self.bottom
    }
    pub fn left(&self) -> Option<PaddingOffset> {
        self.left
    }
    pub fn right(&self) -> Option<PaddingOffset> {
        self.right
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PartialBorderConfig {
    size: Option<BorderSize>,
    color: Option<DrawingColor>,
    radius: Option<BorderRadius>,
}

impl PartialBorderConfig {
    pub fn new(
        size: Option<BorderSize>,
        color: Option<DrawingColor>,
        radius: Option<BorderRadius>,
    ) -> Self {
        Self {
            size,
            color,
            radius,
        }
    }

    pub fn size(&self) -> Option<BorderSize> {
        self.size
    }
    pub fn color(&self) -> Option<&DrawingColor> {
        self.color.as_ref()
    }
    pub fn radius(&self) -> Option<BorderRadius> {
        self.radius
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PartialBarConfig {
    background: Option<DrawingColor>,
    height: Option<u32>,
    vertical_alignment: Option<VerticalAlignment>,
    border: Option<PartialBorderConfig>,
    margin: Option<PartialMarginConfig>,
    padding: Option<PartialPaddingConfig>,
    font_family: Option<FontFamily>,
    font_size: Option<FontSize>,
}

impl PartialBarConfig {
    pub fn new(
        background: Option<DrawingColor>,
        height: Option<u32>,
        vertical_alignment: Option<VerticalAlignment>,
        border: Option<PartialBorderConfig>,
        margin: Option<PartialMarginConfig>,
        padding: Option<PartialPaddingConfig>,
        font_family: Option<FontFamily>,
        font_size: Option<FontSize>,
    ) -> Self {
        Self {
            background,
            height,
            vertical_alignment,
            border,
            margin,
            padding,
            font_family,
            font_size,
        }
    }

    pub fn background(&self) -> Option<&DrawingColor> {
        self.background.as_ref()
    }
    pub fn height(&self) -> Option<u32> {
        self.height
    }
    pub fn vertical_alignment(&self) -> Option<VerticalAlignment> {
        self.vertical_alignment
    }
    pub fn border(&self) -> Option<&PartialBorderConfig> {
        self.border.as_ref()
    }
    pub fn margin(&self) -> Option<&PartialMarginConfig> {
        self.margin.as_ref()
    }
    pub fn padding(&self) -> Option<&PartialPaddingConfig> {
        self.padding.as_ref()
    }
    pub fn font_family(&self) -> Option<&FontFamily> {
        self.font_family.as_ref()
    }
    pub fn font_size(&self) -> Option<FontSize> {
        self.font_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_family() {
        let f = FontFamily::new("Inter".into());
        assert_eq!(f.as_str(), "Inter");
    }

    #[test]
    fn test_font_size() {
        let s = FontSize::new(12.5);
        assert_eq!(s.value(), 12.5);
    }

    #[test]
    fn test_border_size() {
        let s = BorderSize::new(2.0);
        assert_eq!(s.value(), 2.0);
    }

    #[test]
    fn test_border_radius() {
        let r = BorderRadius::new(5.0);
        assert_eq!(r.value(), 5.0);
    }

    #[test]
    fn test_padding_offset() {
        let p = PaddingOffset::new(10);
        assert_eq!(p.value(), 10);
    }

    #[test]
    fn test_margin_offset() {
        let m = MarginOffset::new(15);
        assert_eq!(m.value(), 15);
    }

    #[test]
    fn test_margin_config() {
        let m = MarginConfig::new(
            MarginOffset::new(1),
            MarginOffset::new(2),
            MarginOffset::new(3),
            MarginOffset::new(4)
        );
        assert_eq!(m.top().value(), 1);
        assert_eq!(m.bottom().value(), 2);
        assert_eq!(m.left().value(), 3);
        assert_eq!(m.right().value(), 4);
    }

    #[test]
    fn test_padding_config() {
        let p = PaddingConfig::new(
            PaddingOffset::new(1),
            PaddingOffset::new(2),
            PaddingOffset::new(3),
            PaddingOffset::new(4)
        );
        assert_eq!(p.top().value(), 1);
        assert_eq!(p.bottom().value(), 2);
        assert_eq!(p.left().value(), 3);
        assert_eq!(p.right().value(), 4);
    }

    #[test]
    fn test_border_config() {
        let c = DrawingColor::parse("#ff0000").unwrap();
        let b = BorderConfig::new(
            BorderSize::new(2.0),
            c.clone(),
            BorderRadius::new(4.0)
        );
        assert_eq!(b.size().value(), 2.0);
        assert_eq!(b.color(), &c);
        assert_eq!(b.radius().value(), 4.0);
    }

    #[test]
    fn test_bar_config_defaults() {
        let bar = BarConfig::default();
        assert_eq!(bar.height(), 30);
        assert_eq!(bar.vertical_alignment(), VerticalAlignment::Center);
        assert_eq!(bar.font_family().as_str(), "");
        assert_eq!(bar.font_size().value(), 14.0);
        
        let unfocused = bar.as_unfocused();
        assert_eq!(unfocused.height(), 30);
    }

    #[test]
    fn test_module_position() {
        let left = vec![ModuleConfig::new("time".to_string(), true, HashMap::new())];
        let modules = ModulesConfig::new(left, vec![], vec![]);
        
        let config = Config::new(BarConfig::default(), modules, RenderingMode::default(), crate::domain::metrics::MetricsConfig::default());
        assert_eq!(config.modules().left().len(), 1);
        assert_eq!(config.modules().left()[0].name(), "time");
        assert_eq!(config.modules().center().len(), 0);
        assert_eq!(config.modules().right().len(), 0);
    }
}

