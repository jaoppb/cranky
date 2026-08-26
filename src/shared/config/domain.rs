use crate::shared::primitives::color::DrawingColor;
use crate::shared::primitives::geometry::BarHeight;
use crate::shared::primitives::{ModuleName, ModuleOptions};
use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FontFamily(String);

impl FontFamily {
    #[must_use]
    pub const fn new(family: String) -> Self {
        Self(family)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct FontSize(f32);

impl FontSize {
    #[must_use]
    pub const fn new(size: f32) -> Self {
        Self(size)
    }

    #[must_use]
    pub const fn value(&self) -> f32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BorderSize(f32);

impl BorderSize {
    #[must_use]
    pub const fn new(size: f32) -> Self {
        Self(size)
    }

    #[must_use]
    pub const fn value(&self) -> f32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct BorderRadius(f32);

impl BorderRadius {
    #[must_use]
    pub const fn new(radius: f32) -> Self {
        Self(radius)
    }

    #[must_use]
    pub const fn value(&self) -> f32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MarginOffset(i32);

impl MarginOffset {
    #[must_use]
    pub const fn new(offset: i32) -> Self {
        Self(offset)
    }

    #[must_use]
    pub const fn value(&self) -> i32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PaddingOffset(u32);

impl PaddingOffset {
    #[must_use]
    pub const fn new(offset: u32) -> Self {
        Self(offset)
    }

    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Config {
    root: RootConfig,
    modules: ModulesConfig,
    rendering: RenderingMode,
    metrics: crate::features::metrics::domain::MetricsConfig,
    tooltip: TooltipConfig,
}

impl Config {
    #[must_use]
    pub const fn new(
        root: RootConfig,
        modules: ModulesConfig,
        rendering: RenderingMode,
        metrics: crate::features::metrics::domain::MetricsConfig,
        tooltip: TooltipConfig,
    ) -> Self {
        Self {
            root,
            modules,
            rendering,
            metrics,
            tooltip,
        }
    }

    #[must_use]
    pub const fn root(&self) -> &RootConfig {
        &self.root
    }

    #[must_use]
    pub const fn modules(&self) -> &ModulesConfig {
        &self.modules
    }

    #[must_use]
    pub const fn metrics(&self) -> &crate::features::metrics::domain::MetricsConfig {
        &self.metrics
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TooltipConfig {
    background: DrawingColor,
    border_color: DrawingColor,
    text_color: DrawingColor,
    font: Option<FontFamily>,
    size: Option<FontSize>,
    radius: BorderRadius,
    border_width: BorderSize,
    padding: PaddingOffset,
}

impl TooltipConfig {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub const fn new(
        background: DrawingColor,
        border_color: DrawingColor,
        text_color: DrawingColor,
        font: Option<FontFamily>,
        size: Option<FontSize>,
        radius: BorderRadius,
        border_width: BorderSize,
        padding: PaddingOffset,
    ) -> Self {
        Self {
            background,
            border_color,
            text_color,
            font,
            size,
            radius,
            border_width,
            padding,
        }
    }

    #[must_use]
    pub const fn background(&self) -> &DrawingColor {
        &self.background
    }
    #[must_use]
    pub const fn border_color(&self) -> &DrawingColor {
        &self.border_color
    }
    #[must_use]
    pub const fn text_color(&self) -> &DrawingColor {
        &self.text_color
    }
    #[must_use]
    pub const fn font(&self) -> Option<&FontFamily> {
        self.font.as_ref()
    }
    #[must_use]
    pub const fn size(&self) -> Option<FontSize> {
        self.size
    }
    #[must_use]
    pub const fn radius(&self) -> BorderRadius {
        self.radius
    }
    #[must_use]
    pub const fn border_width(&self) -> BorderSize {
        self.border_width
    }
    #[must_use]
    pub const fn padding(&self) -> PaddingOffset {
        self.padding
    }
}

impl Default for TooltipConfig {
    fn default() -> Self {
        Self {
            background: DrawingColor::Solid(crate::shared::primitives::color::Color::new(
                0x1e, 0x1e, 0x2e, 255,
            )),
            border_color: DrawingColor::Solid(crate::shared::primitives::color::Color::new(
                0xc0, 0xca, 0xf5, 255,
            )),
            text_color: DrawingColor::Solid(crate::shared::primitives::color::Color::new(
                0xc0, 0xca, 0xf5, 255,
            )),
            font: Some(FontFamily::new("Inter".to_string())),
            size: Some(FontSize::new(12.0)),
            radius: BorderRadius::new(4.0),
            border_width: BorderSize::new(1.0),
            padding: PaddingOffset::new(8),
        }
    }
}

impl Default for RenderingMode {
    fn default() -> Self {
        Self::Timebased { duration_ms: 100 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderingMode {
    Immediate { fps_limit: Option<u32> },
    Timebased { duration_ms: u64 },
}

impl RenderingMode {
    #[must_use]
    pub const fn new_immediate(fps_limit: Option<u32>) -> Self {
        Self::Immediate { fps_limit }
    }

    #[must_use]
    pub const fn new_timebased(duration_ms: u64) -> Self {
        Self::Timebased { duration_ms }
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
pub struct RootConfig {
    name: ModuleName,
    height: BarHeight,
    vertical_alignment: VerticalAlignment,
    margin: MarginConfig,
    unfocused: Option<PartialRootConfig>,
    options: ModuleOptions,
}

pub struct CreateRootConfigCommand {
    name: ModuleName,
    height: BarHeight,
    vertical_alignment: VerticalAlignment,
    margin: MarginConfig,
    unfocused: Option<PartialRootConfig>,
    options: ModuleOptions,
}

impl CreateRootConfigCommand {
    #[must_use]
    pub const fn new(
        name: ModuleName,
        height: BarHeight,
        vertical_alignment: VerticalAlignment,
        margin: MarginConfig,
        unfocused: Option<PartialRootConfig>,
        options: ModuleOptions,
    ) -> Self {
        Self {
            name,
            height,
            vertical_alignment,
            margin,
            unfocused,
            options,
        }
    }

    #[must_use]
    pub const fn name(&self) -> &ModuleName {
        &self.name
    }
    #[must_use]
    pub const fn height(&self) -> BarHeight {
        self.height
    }
    #[must_use]
    pub const fn vertical_alignment(&self) -> VerticalAlignment {
        self.vertical_alignment
    }
    #[must_use]
    pub const fn margin(&self) -> &MarginConfig {
        &self.margin
    }
    #[must_use]
    pub const fn unfocused(&self) -> Option<&PartialRootConfig> {
        self.unfocused.as_ref()
    }
    #[must_use]
    pub const fn options(&self) -> &ModuleOptions {
        &self.options
    }
}

impl Default for RootConfig {
    fn default() -> Self {
        Self {
            name: ModuleName::new("bar"),
            height: BarHeight::new(30),
            vertical_alignment: VerticalAlignment::default(),
            margin: MarginConfig::default(),
            unfocused: None,
            options: ModuleOptions::default(),
        }
    }
}

impl RootConfig {
    #[allow(clippy::needless_pass_by_value)]
    #[must_use]
    pub fn new(cmd: CreateRootConfigCommand) -> Self {
        Self {
            name: cmd.name().clone(),
            height: cmd.height(),
            vertical_alignment: cmd.vertical_alignment(),
            margin: cmd.margin().clone(),
            unfocused: cmd.unfocused().cloned(),
            options: cmd.options().clone(),
        }
    }

    #[must_use]
    pub const fn name(&self) -> &ModuleName {
        &self.name
    }

    #[must_use]
    pub const fn height(&self) -> BarHeight {
        self.height
    }

    #[cfg(test)]
    #[must_use]
    pub const fn vertical_alignment(&self) -> VerticalAlignment {
        self.vertical_alignment
    }

    #[must_use]
    pub const fn margin(&self) -> &MarginConfig {
        &self.margin
    }

    #[must_use]
    pub const fn options(&self) -> &ModuleOptions {
        &self.options
    }

    #[must_use]
    pub fn as_unfocused(&self) -> Self {
        let mut base = self.clone();
        if let Some(unfocused) = &self.unfocused {
            if let Some(h) = unfocused.height() {
                base.height = h;
            }
            if let Some(va) = unfocused.vertical_alignment() {
                base.vertical_alignment = va;
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
        }
        base
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MarginConfig {
    top: MarginOffset,
    bottom: MarginOffset,
    left: MarginOffset,
    right: MarginOffset,
}

impl MarginConfig {
    #[must_use]
    pub const fn new(
        top: MarginOffset,
        bottom: MarginOffset,
        left: MarginOffset,
        right: MarginOffset,
    ) -> Self {
        Self {
            top,
            bottom,
            left,
            right,
        }
    }

    #[must_use]
    pub const fn top(&self) -> MarginOffset {
        self.top
    }

    #[must_use]
    pub const fn bottom(&self) -> MarginOffset {
        self.bottom
    }

    #[must_use]
    pub const fn left(&self) -> MarginOffset {
        self.left
    }

    #[must_use]
    pub const fn right(&self) -> MarginOffset {
        self.right
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModulesConfig {
    modules: HashMap<ModuleName, ModuleConfig>,
}

impl ModulesConfig {
    #[must_use]
    pub const fn new(modules: HashMap<ModuleName, ModuleConfig>) -> Self {
        Self { modules }
    }

    #[must_use]
    pub fn get(&self, name: &ModuleName) -> Option<&ModuleConfig> {
        self.modules.get(name)
    }

    #[must_use]
    pub const fn modules(&self) -> &HashMap<ModuleName, ModuleConfig> {
        &self.modules
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EngineId(String);

impl EngineId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EngineId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileExtension(String);

impl FileExtension {
    #[must_use]
    pub fn new(ext: impl Into<String>) -> Self {
        Self(ext.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for FileExtension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum EngineSelection {
    #[default]
    Auto,
    Explicit(EngineId),
}

impl EngineSelection {
    #[cfg(test)]
    #[must_use]
    pub const fn is_auto(&self) -> bool {
        matches!(self, Self::Auto)
    }

    #[must_use]
    pub const fn as_explicit(&self) -> Option<&EngineId> {
        match self {
            Self::Explicit(id) => Some(id),
            Self::Auto => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleConfig {
    name: ModuleName,
    enable: bool,
    engine: EngineSelection,
    options: ModuleOptions,
}

impl ModuleConfig {
    #[must_use]
    pub const fn new(
        name: ModuleName,
        enable: bool,
        engine: EngineSelection,
        options: ModuleOptions,
    ) -> Self {
        Self {
            name,
            enable,
            engine,
            options,
        }
    }

    #[must_use]
    pub const fn name(&self) -> &ModuleName {
        &self.name
    }

    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.enable
    }

    #[must_use]
    pub const fn engine(&self) -> &EngineSelection {
        &self.engine
    }

    #[must_use]
    pub const fn options(&self) -> &ModuleOptions {
        &self.options
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PartialMarginConfig {
    top: Option<MarginOffset>,
    bottom: Option<MarginOffset>,
    left: Option<MarginOffset>,
    right: Option<MarginOffset>,
}

impl PartialMarginConfig {
    #[must_use]
    pub const fn new(
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

    #[must_use]
    pub const fn top(&self) -> Option<MarginOffset> {
        self.top
    }
    #[must_use]
    pub const fn bottom(&self) -> Option<MarginOffset> {
        self.bottom
    }
    #[must_use]
    pub const fn left(&self) -> Option<MarginOffset> {
        self.left
    }
    #[must_use]
    pub const fn right(&self) -> Option<MarginOffset> {
        self.right
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PartialRootConfig {
    height: Option<BarHeight>,
    vertical_alignment: Option<VerticalAlignment>,
    margin: Option<PartialMarginConfig>,
}

pub struct CreatePartialRootConfigCommand {
    height: Option<BarHeight>,
    vertical_alignment: Option<VerticalAlignment>,
    margin: Option<PartialMarginConfig>,
}

impl CreatePartialRootConfigCommand {
    #[must_use]
    pub const fn new(
        height: Option<BarHeight>,
        vertical_alignment: Option<VerticalAlignment>,
        margin: Option<PartialMarginConfig>,
    ) -> Self {
        Self {
            height,
            vertical_alignment,
            margin,
        }
    }

    #[must_use]
    pub const fn height(&self) -> Option<BarHeight> {
        self.height
    }
    #[must_use]
    pub const fn vertical_alignment(&self) -> Option<VerticalAlignment> {
        self.vertical_alignment
    }
    #[must_use]
    pub const fn margin(&self) -> Option<&PartialMarginConfig> {
        self.margin.as_ref()
    }
}

impl PartialRootConfig {
    #[allow(clippy::needless_pass_by_value)]
    #[must_use]
    pub fn new(cmd: CreatePartialRootConfigCommand) -> Self {
        Self {
            height: cmd.height(),
            vertical_alignment: cmd.vertical_alignment(),
            margin: cmd.margin().cloned(),
        }
    }

    #[must_use]
    pub const fn height(&self) -> Option<BarHeight> {
        self.height
    }
    #[must_use]
    pub const fn vertical_alignment(&self) -> Option<VerticalAlignment> {
        self.vertical_alignment
    }
    #[must_use]
    pub const fn margin(&self) -> Option<&PartialMarginConfig> {
        self.margin.as_ref()
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
        assert!((s.value() - 12.5).abs() < f32::EPSILON);
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
            MarginOffset::new(4),
        );
        assert_eq!(m.top().value(), 1);
        assert_eq!(m.bottom().value(), 2);
        assert_eq!(m.left().value(), 3);
        assert_eq!(m.right().value(), 4);
    }

    #[test]
    fn test_root_config_defaults() {
        let root = RootConfig::default();
        assert_eq!(root.name().as_str(), "bar");
        assert_eq!(root.height().value(), 30);
        assert_eq!(root.vertical_alignment(), VerticalAlignment::Center);

        let unfocused = root.as_unfocused();
        assert_eq!(unfocused.height().value(), 30);
    }

    #[test]
    fn test_modules_config() {
        let mut modules_map = HashMap::new();
        modules_map.insert(
            ModuleName::new("time"),
            ModuleConfig::new(
                ModuleName::new("time"),
                true,
                EngineSelection::Auto,
                ModuleOptions::default(),
            ),
        );
        let modules = ModulesConfig::new(modules_map);

        let config = Config::new(
            RootConfig::default(),
            modules,
            RenderingMode::default(),
            crate::features::metrics::domain::MetricsConfig::default(),
            TooltipConfig::default(),
        );
        assert!(config.modules().get(&ModuleName::new("time")).is_some());
        assert_eq!(
            config
                .modules()
                .get(&ModuleName::new("time"))
                .unwrap()
                .name(),
            "time"
        );
    }

    #[test]
    fn test_module_config_engine() {
        let explicit = EngineSelection::Explicit(EngineId::new("rhai"));
        let cfg = ModuleConfig::new(
            "hour".into(),
            true,
            explicit.clone(),
            ModuleOptions::default(),
        );
        assert_eq!(cfg.engine(), &explicit);
        assert_eq!(
            cfg.engine().as_explicit().map(super::EngineId::as_str),
            Some("rhai")
        );
        assert!(!cfg.engine().is_auto());
        assert_eq!(cfg.name(), "hour");
        assert!(cfg.is_enabled());
        assert!(cfg.options().is_empty());

        let auto = EngineSelection::default();
        assert!(auto.is_auto());
        assert_eq!(auto.as_explicit(), None);
    }
}
