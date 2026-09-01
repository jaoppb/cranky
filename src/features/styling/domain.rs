use crate::features::layout_engine::domain::{
    AlignItems, BoxMargin, FlexDirection, Gap, JustifyContent, PositionType,
};
use crate::shared::config::domain::{BorderRadius, BorderSize, FontFamily, FontSize};
use crate::shared::primitives::color::DrawingColor;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum StylingError {
    #[error(
        "Invalid stylesheet name '{0}': must be non-empty alphanumeric with '-' or '_', without path separators or extensions"
    )]
    InvalidStyleSheetName(String),
    #[error("Invalid class name '{0}': must be a valid CSS identifier")]
    InvalidClassName(String),
    #[error("Invalid element ID '{0}': must be a valid CSS identifier")]
    InvalidElementId(String),
    #[error("Invalid progress value {0}: must be within range [0.0, 1.0]")]
    InvalidProgressValue(String),
    #[error("Invalid opacity value {0}: must be within range [0.0, 1.0]")]
    InvalidOpacity(String),
    #[error("Invalid flex value: {0}")]
    InvalidFlexValue(String),
    #[error("Invalid CSS length: {0}")]
    InvalidLength(String),
    #[error("CSS parser error: {0}")]
    ParserError(String),
    #[error("Style loader error: {0}")]
    LoaderError(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StyleSheetName(String);

impl StyleSheetName {
    /// Creates a new `StyleSheetName`.
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidStyleSheetName` if name is empty, contains path separators, `.css` extension, or invalid characters.
    pub fn new(name: impl Into<String>) -> Result<Self, StylingError> {
        let s = name.into();
        if s.is_empty() {
            return Err(StylingError::InvalidStyleSheetName(s));
        }
        if s.contains('/')
            || s.contains('\\')
            || s.contains("..")
            || std::path::Path::new(&s)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("css"))
        {
            return Err(StylingError::InvalidStyleSheetName(s));
        }
        if !s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(StylingError::InvalidStyleSheetName(s));
        }
        Ok(Self(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for StyleSheetName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct ClassName(String);

impl ClassName {
    /// Creates a new `ClassName`.
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidClassName` if name is empty or contains invalid characters.
    pub fn new(name: impl Into<String>) -> Result<Self, StylingError> {
        let s = name.into();
        if s.is_empty() {
            return Err(StylingError::InvalidClassName(s));
        }
        let Some(first) = s.chars().next() else {
            return Err(StylingError::InvalidClassName(s));
        };
        if !first.is_ascii_alphabetic() && first != '_' && first != '-' {
            return Err(StylingError::InvalidClassName(s));
        }
        if !s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(StylingError::InvalidClassName(s));
        }
        Ok(Self(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ClassName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'de> Deserialize<'de> for ClassName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::new(s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize)]
pub struct ClassNameList(Vec<ClassName>);

impl ClassNameList {
    /// Parses a space-separated string of class names.
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidClassName` if any class name is invalid.
    pub fn parse(classes: &str) -> Result<Self, StylingError> {
        let mut list = Vec::new();
        for item in classes.split_whitespace() {
            list.push(ClassName::new(item)?);
        }
        Ok(Self(list))
    }

    #[must_use]
    pub const fn new(list: Vec<ClassName>) -> Self {
        Self(list)
    }

    #[must_use]
    pub fn as_slice(&self) -> &[ClassName] {
        &self.0
    }

    pub fn iter(&self) -> std::slice::Iter<'_, ClassName> {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a ClassNameList {
    type Item = &'a ClassName;
    type IntoIter = std::slice::Iter<'a, ClassName>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'de> Deserialize<'de> for ClassNameList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ClassRepr {
            Str(String),
            List(Vec<String>),
        }

        match ClassRepr::deserialize(deserializer)? {
            ClassRepr::Str(s) => Self::parse(&s).map_err(serde::de::Error::custom),
            ClassRepr::List(list) => {
                let mut out = Vec::new();
                for item in list {
                    out.push(ClassName::new(item).map_err(serde::de::Error::custom)?);
                }
                Ok(Self(out))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct ElementId(String);

impl ElementId {
    /// Creates a new `ElementId`.
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidElementId` if id is empty or contains invalid characters.
    pub fn new(id: impl Into<String>) -> Result<Self, StylingError> {
        let s = id.into();
        if s.is_empty() {
            return Err(StylingError::InvalidElementId(s));
        }
        let Some(first) = s.chars().next() else {
            return Err(StylingError::InvalidElementId(s));
        };
        if !first.is_ascii_alphabetic() && first != '_' {
            return Err(StylingError::InvalidElementId(s));
        }
        if !s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(StylingError::InvalidElementId(s));
        }
        Ok(Self(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ElementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'de> Deserialize<'de> for ElementId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::new(s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Default)]
pub struct ProgressValue(f32);

impl ProgressValue {
    /// Creates a new `ProgressValue` between 0.0 and 1.0.
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidProgressValue` if `value` is NaN or not in 0.0..=1.0.
    pub fn new(value: f32) -> Result<Self, StylingError> {
        if value.is_nan() || !(0.0..=1.0).contains(&value) {
            return Err(StylingError::InvalidProgressValue(format!("{value}")));
        }
        Ok(Self(value))
    }

    /// Creates a new `ProgressValue` from a percentage value (0.0 to 100.0).
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidProgressValue` if `pct` is NaN or not in 0.0..=100.0.
    pub fn from_percentage(pct: f32) -> Result<Self, StylingError> {
        Self::new(pct / 100.0)
    }

    #[must_use]
    pub const fn value(&self) -> f32 {
        self.0
    }
}

impl<'de> Deserialize<'de> for ProgressValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = f32::deserialize(deserializer)?;
        Self::new(v).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

impl Orientation {
    #[must_use]
    pub const fn is_horizontal(&self) -> bool {
        matches!(self, Self::Horizontal)
    }

    #[must_use]
    pub const fn is_vertical(&self) -> bool {
        matches!(self, Self::Vertical)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Opacity(f32);

impl Opacity {
    /// Creates a new `Opacity` between 0.0 and 1.0.
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidOpacity` if `value` is NaN or not in 0.0..=1.0.
    pub fn new(value: f32) -> Result<Self, StylingError> {
        if value.is_nan() || !(0.0..=1.0).contains(&value) {
            return Err(StylingError::InvalidOpacity(format!("{value}")));
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn value(&self) -> f32 {
        self.0
    }
}

impl<'de> Deserialize<'de> for Opacity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = f32::deserialize(deserializer)?;
        Self::new(v).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct FlexGrow(f32);

impl FlexGrow {
    /// Creates a new `FlexGrow` value >= 0.0.
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidFlexValue` if `value` is NaN or negative.
    pub fn new(value: f32) -> Result<Self, StylingError> {
        if value.is_nan() || value < 0.0 {
            return Err(StylingError::InvalidFlexValue(format!(
                "flex-grow cannot be negative or NaN, got {value}"
            )));
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn value(&self) -> f32 {
        self.0
    }
}

impl<'de> Deserialize<'de> for FlexGrow {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = f32::deserialize(deserializer)?;
        Self::new(v).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct FlexShrink(f32);

impl FlexShrink {
    /// Creates a new `FlexShrink` value >= 0.0.
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidFlexValue` if `value` is NaN or negative.
    pub fn new(value: f32) -> Result<Self, StylingError> {
        if value.is_nan() || value < 0.0 {
            return Err(StylingError::InvalidFlexValue(format!(
                "flex-shrink cannot be negative or NaN, got {value}"
            )));
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn value(&self) -> f32 {
        self.0
    }
}

impl<'de> Deserialize<'de> for FlexShrink {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = f32::deserialize(deserializer)?;
        Self::new(v).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CssLength {
    Px(f32),
    Percent(f32),
    Auto,
}

impl CssLength {
    /// Creates a pixel length value.
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidLength` if `v` is NaN or negative.
    pub fn px(v: f32) -> Result<Self, StylingError> {
        if v.is_nan() || v < 0.0 {
            return Err(StylingError::InvalidLength(format!(
                "Length cannot be negative or NaN, got {v}"
            )));
        }
        Ok(Self::Px(v))
    }

    /// Creates a percentage length value.
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidLength` if `v` is NaN or negative.
    pub fn percent(v: f32) -> Result<Self, StylingError> {
        if v.is_nan() || v < 0.0 {
            return Err(StylingError::InvalidLength(format!(
                "Percentage cannot be negative or NaN, got {v}"
            )));
        }
        Ok(Self::Percent(v))
    }

    #[must_use]
    pub const fn value(&self) -> Option<f32> {
        match self {
            Self::Px(v) | Self::Percent(v) => Some(*v),
            Self::Auto => None,
        }
    }

    #[must_use]
    pub const fn is_auto(&self) -> bool {
        matches!(self, Self::Auto)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DisplayMode {
    Flex,
    Grid,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GridAutoFlow {
    #[default]
    Row,
    Column,
    RowDense,
    ColumnDense,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GridTrack {
    Px(f32),
    Percent(f32),
    Fr(f32),
    Auto,
    MinContent,
    MaxContent,
    MinMax(Box<Self>, Box<Self>),
    Repeat(u16, Vec<Self>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum GridPlacement {
    #[default]
    Auto,
    Line(i16),
    Span(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct GridLinePlacement {
    start: GridPlacement,
    end: GridPlacement,
}

impl GridLinePlacement {
    #[must_use]
    pub const fn new(start: GridPlacement, end: GridPlacement) -> Self {
        Self { start, end }
    }

    #[must_use]
    pub const fn start(&self) -> GridPlacement {
        self.start
    }

    #[must_use]
    pub const fn end(&self) -> GridPlacement {
        self.end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PseudoClass {
    Hover,
    Active,
    Focused,
    FirstChild,
    LastChild,
    OnlyChild,
    NthChild { a: i32, b: i32 },
    NthLastChild { a: i32, b: i32 },
    Empty,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ComputedStyle {
    background: Option<DrawingColor>,
    color: Option<DrawingColor>,
    accent_color: Option<DrawingColor>,
    font_family: Option<FontFamily>,
    font_size: Option<FontSize>,
    border_size: Option<BorderSize>,
    border_color: Option<DrawingColor>,
    border_radius: Option<BorderRadius>,
    width: Option<CssLength>,
    height: Option<CssLength>,
    min_width: Option<CssLength>,
    min_height: Option<CssLength>,
    max_width: Option<CssLength>,
    max_height: Option<CssLength>,
    padding: Option<BoxMargin>,
    margin: Option<BoxMargin>,
    gap: Option<Gap>,
    column_gap: Option<Gap>,
    row_gap: Option<Gap>,
    display: Option<DisplayMode>,
    flex_direction: Option<FlexDirection>,
    justify_content: Option<JustifyContent>,
    align_items: Option<AlignItems>,
    justify_items: Option<AlignItems>,
    justify_self: Option<AlignItems>,
    align_content: Option<JustifyContent>,
    grid_template_columns: Option<Vec<GridTrack>>,
    grid_template_rows: Option<Vec<GridTrack>>,
    grid_auto_columns: Option<Vec<GridTrack>>,
    grid_auto_rows: Option<Vec<GridTrack>>,
    grid_auto_flow: Option<GridAutoFlow>,
    grid_column: Option<GridLinePlacement>,
    grid_row: Option<GridLinePlacement>,
    position: Option<PositionType>,
    opacity: Option<Opacity>,
    flex_grow: Option<FlexGrow>,
    flex_shrink: Option<FlexShrink>,
    flex_basis: Option<CssLength>,
    align_self: Option<AlignItems>,
}

impl ComputedStyle {
    #[must_use]
    pub fn default_for_flex() -> Self {
        Self {
            display: Some(DisplayMode::Flex),
            flex_direction: Some(FlexDirection::Row),
            align_items: Some(AlignItems::Start),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn default_for_grid() -> Self {
        Self {
            display: Some(DisplayMode::Grid),
            grid_auto_flow: Some(GridAutoFlow::Row),
            ..Default::default()
        }
    }

    #[must_use]
    pub const fn background(&self) -> Option<&DrawingColor> {
        self.background.as_ref()
    }
    #[must_use]
    pub const fn color(&self) -> Option<&DrawingColor> {
        self.color.as_ref()
    }
    #[must_use]
    pub const fn accent_color(&self) -> Option<&DrawingColor> {
        self.accent_color.as_ref()
    }
    #[must_use]
    pub const fn font_family(&self) -> Option<&FontFamily> {
        self.font_family.as_ref()
    }
    #[must_use]
    pub const fn font_size(&self) -> Option<FontSize> {
        self.font_size
    }
    #[must_use]
    pub const fn border_size(&self) -> Option<BorderSize> {
        self.border_size
    }
    #[must_use]
    pub const fn border_color(&self) -> Option<&DrawingColor> {
        self.border_color.as_ref()
    }
    #[must_use]
    pub const fn border_radius(&self) -> Option<BorderRadius> {
        self.border_radius
    }
    #[must_use]
    pub const fn width(&self) -> Option<CssLength> {
        self.width
    }
    #[must_use]
    pub const fn height(&self) -> Option<CssLength> {
        self.height
    }
    #[must_use]
    pub const fn min_width(&self) -> Option<CssLength> {
        self.min_width
    }
    #[must_use]
    pub const fn min_height(&self) -> Option<CssLength> {
        self.min_height
    }
    #[must_use]
    pub const fn max_width(&self) -> Option<CssLength> {
        self.max_width
    }
    #[must_use]
    pub const fn max_height(&self) -> Option<CssLength> {
        self.max_height
    }
    #[must_use]
    pub const fn padding(&self) -> Option<&BoxMargin> {
        self.padding.as_ref()
    }
    #[must_use]
    pub const fn margin(&self) -> Option<&BoxMargin> {
        self.margin.as_ref()
    }
    #[must_use]
    pub const fn gap(&self) -> Option<&Gap> {
        self.gap.as_ref()
    }
    #[must_use]
    pub const fn column_gap(&self) -> Option<&Gap> {
        self.column_gap.as_ref()
    }
    #[must_use]
    pub const fn row_gap(&self) -> Option<&Gap> {
        self.row_gap.as_ref()
    }
    #[must_use]
    pub const fn display(&self) -> Option<DisplayMode> {
        self.display
    }
    #[must_use]
    pub const fn flex_direction(&self) -> Option<FlexDirection> {
        self.flex_direction
    }
    #[must_use]
    pub const fn justify_content(&self) -> Option<JustifyContent> {
        self.justify_content
    }
    #[must_use]
    pub const fn align_items(&self) -> Option<AlignItems> {
        self.align_items
    }
    #[must_use]
    pub const fn justify_items(&self) -> Option<AlignItems> {
        self.justify_items
    }
    #[must_use]
    pub const fn justify_self(&self) -> Option<AlignItems> {
        self.justify_self
    }
    #[must_use]
    pub const fn align_content(&self) -> Option<JustifyContent> {
        self.align_content
    }
    #[must_use]
    pub fn grid_template_columns(&self) -> Option<&[GridTrack]> {
        self.grid_template_columns.as_deref()
    }
    #[must_use]
    pub fn grid_template_rows(&self) -> Option<&[GridTrack]> {
        self.grid_template_rows.as_deref()
    }
    #[must_use]
    pub fn grid_auto_columns(&self) -> Option<&[GridTrack]> {
        self.grid_auto_columns.as_deref()
    }
    #[must_use]
    pub fn grid_auto_rows(&self) -> Option<&[GridTrack]> {
        self.grid_auto_rows.as_deref()
    }
    #[must_use]
    pub const fn grid_auto_flow(&self) -> Option<GridAutoFlow> {
        self.grid_auto_flow
    }
    #[must_use]
    pub const fn grid_column(&self) -> Option<&GridLinePlacement> {
        self.grid_column.as_ref()
    }
    #[must_use]
    pub const fn grid_row(&self) -> Option<&GridLinePlacement> {
        self.grid_row.as_ref()
    }
    #[must_use]
    pub const fn position(&self) -> Option<PositionType> {
        self.position
    }
    #[must_use]
    pub const fn opacity(&self) -> Option<Opacity> {
        self.opacity
    }
    #[must_use]
    pub const fn flex_grow(&self) -> Option<FlexGrow> {
        self.flex_grow
    }
    #[must_use]
    pub const fn flex_shrink(&self) -> Option<FlexShrink> {
        self.flex_shrink
    }
    #[must_use]
    pub const fn flex_basis(&self) -> Option<CssLength> {
        self.flex_basis
    }
    #[must_use]
    pub const fn align_self(&self) -> Option<AlignItems> {
        self.align_self
    }

    pub fn set_background(&mut self, bg: DrawingColor) {
        self.background = Some(bg);
    }
    pub fn set_color(&mut self, color: DrawingColor) {
        self.color = Some(color);
    }
    pub fn set_accent_color(&mut self, color: DrawingColor) {
        self.accent_color = Some(color);
    }
    pub fn set_font_family(&mut self, font_family: FontFamily) {
        self.font_family = Some(font_family);
    }
    pub const fn set_font_size(&mut self, font_size: FontSize) {
        self.font_size = Some(font_size);
    }
    pub const fn set_border_size(&mut self, border_size: BorderSize) {
        self.border_size = Some(border_size);
    }
    pub fn set_border_color(&mut self, border_color: DrawingColor) {
        self.border_color = Some(border_color);
    }
    pub const fn set_border_radius(&mut self, border_radius: BorderRadius) {
        self.border_radius = Some(border_radius);
    }
    pub const fn set_width(&mut self, width: CssLength) {
        self.width = Some(width);
    }
    pub const fn set_height(&mut self, height: CssLength) {
        self.height = Some(height);
    }
    pub const fn set_min_width(&mut self, min_width: CssLength) {
        self.min_width = Some(min_width);
    }
    pub const fn set_min_height(&mut self, min_height: CssLength) {
        self.min_height = Some(min_height);
    }
    pub const fn set_max_width(&mut self, max_width: CssLength) {
        self.max_width = Some(max_width);
    }
    pub const fn set_max_height(&mut self, max_height: CssLength) {
        self.max_height = Some(max_height);
    }
    pub const fn set_padding(&mut self, padding: BoxMargin) {
        self.padding = Some(padding);
    }
    pub const fn set_margin(&mut self, margin: BoxMargin) {
        self.margin = Some(margin);
    }
    pub const fn set_gap(&mut self, gap: Gap) {
        self.gap = Some(gap);
    }
    pub const fn set_column_gap(&mut self, column_gap: Gap) {
        self.column_gap = Some(column_gap);
    }
    pub const fn set_row_gap(&mut self, row_gap: Gap) {
        self.row_gap = Some(row_gap);
    }
    pub const fn set_display(&mut self, display: DisplayMode) {
        self.display = Some(display);
    }
    pub const fn set_flex_direction(&mut self, flex_direction: FlexDirection) {
        self.flex_direction = Some(flex_direction);
    }
    pub const fn set_justify_content(&mut self, justify_content: JustifyContent) {
        self.justify_content = Some(justify_content);
    }
    pub const fn set_align_items(&mut self, align_items: AlignItems) {
        self.align_items = Some(align_items);
    }
    pub const fn set_justify_items(&mut self, justify_items: AlignItems) {
        self.justify_items = Some(justify_items);
    }
    pub const fn set_justify_self(&mut self, justify_self: AlignItems) {
        self.justify_self = Some(justify_self);
    }
    pub const fn set_align_content(&mut self, align_content: JustifyContent) {
        self.align_content = Some(align_content);
    }
    pub fn set_grid_template_columns(&mut self, grid_template_columns: Vec<GridTrack>) {
        self.grid_template_columns = Some(grid_template_columns);
    }
    pub fn set_grid_template_rows(&mut self, grid_template_rows: Vec<GridTrack>) {
        self.grid_template_rows = Some(grid_template_rows);
    }
    pub fn set_grid_auto_columns(&mut self, grid_auto_columns: Vec<GridTrack>) {
        self.grid_auto_columns = Some(grid_auto_columns);
    }
    pub fn set_grid_auto_rows(&mut self, grid_auto_rows: Vec<GridTrack>) {
        self.grid_auto_rows = Some(grid_auto_rows);
    }
    pub const fn set_grid_auto_flow(&mut self, grid_auto_flow: GridAutoFlow) {
        self.grid_auto_flow = Some(grid_auto_flow);
    }
    pub const fn set_grid_column(&mut self, grid_column: GridLinePlacement) {
        self.grid_column = Some(grid_column);
    }
    pub const fn set_grid_row(&mut self, grid_row: GridLinePlacement) {
        self.grid_row = Some(grid_row);
    }
    pub const fn set_position(&mut self, position: PositionType) {
        self.position = Some(position);
    }
    pub const fn set_opacity(&mut self, opacity: Opacity) {
        self.opacity = Some(opacity);
    }
    pub const fn set_flex_grow(&mut self, flex_grow: FlexGrow) {
        self.flex_grow = Some(flex_grow);
    }
    pub const fn set_flex_shrink(&mut self, flex_shrink: FlexShrink) {
        self.flex_shrink = Some(flex_shrink);
    }
    pub const fn set_flex_basis(&mut self, flex_basis: CssLength) {
        self.flex_basis = Some(flex_basis);
    }
    pub const fn set_align_self(&mut self, align_self: AlignItems) {
        self.align_self = Some(align_self);
    }

    pub fn merge_with(&mut self, other: &Self) {
        macro_rules! merge_fields {
            ($($field:ident),* $(,)?) => {
                $(
                    if other.$field.is_some() {
                        self.$field.clone_from(&other.$field);
                    }
                )*
            };
        }

        merge_fields!(
            background,
            color,
            accent_color,
            font_family,
            font_size,
            border_size,
            border_color,
            border_radius,
            width,
            height,
            min_width,
            min_height,
            max_width,
            max_height,
            padding,
            margin,
            gap,
            column_gap,
            row_gap,
            display,
            flex_direction,
            justify_content,
            align_items,
            justify_items,
            justify_self,
            align_content,
            grid_template_columns,
            grid_template_rows,
            grid_auto_columns,
            grid_auto_rows,
            grid_auto_flow,
            grid_column,
            grid_row,
            position,
            opacity,
            flex_grow,
            flex_shrink,
            flex_basis,
            align_self,
        );
    }
}

#[derive(Debug, Clone)]
pub struct ElementQuery<'a> {
    tag: &'a str,
    id: Option<&'a ElementId>,
    classes: &'a [ClassName],
    pseudo_classes: &'a [PseudoClass],
    child_index: usize,
    total_children: usize,
    is_empty: bool,
    parent: Option<&'a Self>,
}

impl<'a> ElementQuery<'a> {
    #[must_use]
    pub const fn new(
        tag: &'a str,
        id: Option<&'a ElementId>,
        classes: &'a [ClassName],
        pseudo_classes: &'a [PseudoClass],
        parent: Option<&'a Self>,
    ) -> Self {
        Self {
            tag,
            id,
            classes,
            pseudo_classes,
            child_index: 0,
            total_children: 1,
            is_empty: false,
            parent,
        }
    }

    #[must_use]
    pub const fn with_structural_context(
        mut self,
        child_index: usize,
        total_children: usize,
        is_empty: bool,
    ) -> Self {
        self.child_index = child_index;
        self.total_children = total_children;
        self.is_empty = is_empty;
        self
    }

    #[must_use]
    pub const fn tag(&self) -> &'a str {
        self.tag
    }

    #[must_use]
    pub const fn id(&self) -> Option<&'a ElementId> {
        self.id
    }

    #[must_use]
    pub const fn classes(&self) -> &'a [ClassName] {
        self.classes
    }

    #[must_use]
    pub const fn pseudo_classes(&self) -> &'a [PseudoClass] {
        self.pseudo_classes
    }

    #[must_use]
    pub const fn child_index(&self) -> usize {
        self.child_index
    }

    #[must_use]
    pub const fn total_children(&self) -> usize {
        self.total_children
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.is_empty
    }

    #[must_use]
    pub const fn parent(&self) -> Option<&'a Self> {
        self.parent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stylesheet_name_validation() {
        assert!(StyleSheetName::new("base").is_ok());
        assert!(StyleSheetName::new("hour_style-1").is_ok());
        assert_eq!(
            StyleSheetName::new("").unwrap_err(),
            StylingError::InvalidStyleSheetName(String::new())
        );
        assert!(StyleSheetName::new("base.css").is_err());
        assert!(StyleSheetName::new("../base").is_err());
        assert!(StyleSheetName::new("styles/base").is_err());
        assert!(StyleSheetName::new("base name").is_err());
    }

    #[test]
    fn test_class_name_validation() {
        assert!(ClassName::new("workspace-btn").is_ok());
        assert!(ClassName::new("_active").is_ok());
        assert!(ClassName::new("item1").is_ok());
        assert!(ClassName::new("").is_err());
        assert!(ClassName::new("123item").is_err());
        assert!(ClassName::new("btn.active").is_err());
        assert!(ClassName::new("btn active").is_err());
    }

    #[test]
    fn test_element_id_validation() {
        assert!(ElementId::new("hour-main").is_ok());
        assert!(ElementId::new("ws_1").is_ok());
        assert!(ElementId::new("").is_err());
        assert!(ElementId::new("1ws").is_err());
        assert!(ElementId::new("#ws").is_err());
    }

    #[test]
    fn test_progress_value_validation() {
        assert!(ProgressValue::new(0.0).is_ok());
        assert!(ProgressValue::new(0.5).is_ok());
        assert!(ProgressValue::new(1.0).is_ok());
        assert!(ProgressValue::new(-0.01).is_err());
        assert!(ProgressValue::new(1.01).is_err());
        assert!(ProgressValue::new(f32::NAN).is_err());

        let from_pct = ProgressValue::from_percentage(75.0).unwrap();
        assert!((from_pct.value() - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn test_computed_style_merge() {
        let mut style1 = ComputedStyle::default();
        style1.set_font_size(FontSize::new(12.0));

        let mut style2 = ComputedStyle::default();
        style2.set_font_size(FontSize::new(16.0));
        style2.set_border_radius(BorderRadius::new(4.0));
        style2.set_opacity(Opacity::new(0.8).unwrap());
        style2.set_flex_grow(FlexGrow::new(1.0).unwrap());
        style2.set_flex_shrink(FlexShrink::new(0.0).unwrap());

        style1.merge_with(&style2);
        assert!((style1.font_size().unwrap().value() - 16.0).abs() < f32::EPSILON);
        assert!((style1.border_radius().unwrap().value() - 4.0).abs() < f32::EPSILON);
        assert!((style1.opacity().unwrap().value() - 0.8).abs() < f32::EPSILON);
        assert!((style1.flex_grow().unwrap().value() - 1.0).abs() < f32::EPSILON);
        assert!((style1.flex_shrink().unwrap().value()).abs() < f32::EPSILON);
    }

    #[test]
    fn test_opacity_validation() {
        assert!(Opacity::new(0.0).is_ok());
        assert!(Opacity::new(0.5).is_ok());
        assert!(Opacity::new(1.0).is_ok());
        assert!(Opacity::new(-0.1).is_err());
        assert!(Opacity::new(1.1).is_err());
        assert!(Opacity::new(f32::NAN).is_err());
    }

    #[test]
    fn test_flex_vos_validation() {
        assert!(FlexGrow::new(0.0).is_ok());
        assert!(FlexGrow::new(1.5).is_ok());
        assert!(FlexGrow::new(-1.0).is_err());
        assert!(FlexGrow::new(f32::NAN).is_err());

        assert!(FlexShrink::new(0.0).is_ok());
        assert!(FlexShrink::new(2.0).is_ok());
        assert!(FlexShrink::new(-0.5).is_err());
        assert!(FlexShrink::new(f32::NAN).is_err());
    }
}
