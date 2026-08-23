use crate::features::layout_engine::domain::{
    AlignItems, BoxMargin, FlexDirection, Gap, JustifyContent, PositionType,
};
use crate::features::styling::domain::{
    ComputedStyle, CssLength, ElementQuery, FlexGrow, FlexShrink, Opacity, PseudoClass,
    StyleSheetName, StylingError,
};
use crate::features::styling::ports::{CssParserPort, ParsedStyleSheetPort};
use crate::shared::config::domain::{BorderRadius, BorderSize, FontFamily, FontSize};
use crate::shared::primitives::color::{Color, DrawingColor};
use lightningcss::declaration::DeclarationBlock;
use lightningcss::properties::Property;
use lightningcss::properties::border::Border;
use lightningcss::properties::font::{AbsoluteFontSize, FontSize as LightningFontSize};
use lightningcss::rules::CssRule;
use lightningcss::selector::{
    Combinator, Component, PseudoClass as LightningPseudoClass, Selector,
};
use lightningcss::stylesheet::{ParserOptions, PrinterOptions, StyleSheet};
use lightningcss::traits::ToCss;
use lightningcss::values::color::CssColor as LightningCssColor;
use lightningcss::values::length::LengthPercentageOrAuto;

#[derive(Debug, Clone)]
struct RuleEntry {
    selectors: Vec<CompiledSelector>,
    style: ComputedStyle,
}

#[derive(Debug, Clone)]
struct CompiledSelector {
    steps: Vec<SelectorStep>,
}

#[derive(Debug, Clone)]
struct SelectorStep {
    tag: Option<String>,
    id: Option<String>,
    classes: Vec<String>,
    pseudo_classes: Vec<PseudoClass>,
    combinator: Option<Combinator>,
}

pub struct LightningCssAdapter;

impl LightningCssAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LightningCssAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl CssParserPort for LightningCssAdapter {
    fn parse_stylesheet(
        &self,
        name: StyleSheetName,
        css_source: &str,
    ) -> Result<Box<dyn ParsedStyleSheetPort>, StylingError> {
        tracing::debug!(stylesheet = %name.as_str(), len = css_source.len(), "Parsing CSS stylesheet");
        let stylesheet = StyleSheet::parse(css_source, ParserOptions::default()).map_err(|e| {
            tracing::error!(stylesheet = %name.as_str(), error = ?e, "Failed to parse CSS stylesheet");
            StylingError::ParserError(e.to_string())
        })?;

        let mut rule_entries = Vec::new();

        for rule in &stylesheet.rules.0 {
            if let CssRule::Style(style_rule) = rule {
                let computed = parse_declarations(&style_rule.declarations);
                let mut selectors = Vec::new();
                for sel in &style_rule.selectors.0 {
                    selectors.push(compile_selector(sel));
                }
                rule_entries.push(RuleEntry {
                    selectors,
                    style: computed,
                });
            }
        }

        tracing::debug!(stylesheet = %name.as_str(), rule_count = rule_entries.len(), "CSS stylesheet parsed successfully");

        Ok(Box::new(LightningParsedStyleSheet {
            name,
            rules: rule_entries,
        }))
    }
}

pub struct LightningParsedStyleSheet {
    name: StyleSheetName,
    rules: Vec<RuleEntry>,
}

impl ParsedStyleSheetPort for LightningParsedStyleSheet {
    fn name(&self) -> &StyleSheetName {
        &self.name
    }

    fn resolve_style(&self, query: &ElementQuery) -> ComputedStyle {
        let mut result = ComputedStyle::default();

        for rule in &self.rules {
            let matches = rule
                .selectors
                .iter()
                .any(|sel| matches_selector(sel, query));
            if matches {
                result.merge_with(&rule.style);
            }
        }

        tracing::trace!(
            stylesheet = %self.name.as_str(),
            tag = %query.tag(),
            id = ?query.id().map(|i| i.as_str()),
            classes = ?query.classes().iter().map(|c| c.as_str()).collect::<Vec<_>>(),
            has_bg = result.background().is_some(),
            has_color = result.color().is_some(),
            "Resolved style for query"
        );

        result
    }
}

fn compile_selector(selector: &Selector) -> CompiledSelector {
    let mut steps = Vec::new();
    let mut current_step = SelectorStep {
        tag: None,
        id: None,
        classes: Vec::new(),
        pseudo_classes: Vec::new(),
        combinator: None,
    };

    for component in selector.iter_raw_match_order() {
        match component {
            Component::LocalName(local_name) => {
                current_step.tag = Some(local_name.name.as_ref().to_lowercase());
            }
            Component::ID(id) => {
                current_step.id = Some(id.as_ref().to_string());
            }
            Component::Class(class) => {
                current_step.classes.push(class.as_ref().to_string());
            }
            Component::NonTSPseudoClass(pseudo) => match pseudo {
                LightningPseudoClass::Hover => current_step.pseudo_classes.push(PseudoClass::Hover),
                LightningPseudoClass::Active => {
                    current_step.pseudo_classes.push(PseudoClass::Active)
                }
                LightningPseudoClass::Focus | LightningPseudoClass::FocusVisible => {
                    current_step.pseudo_classes.push(PseudoClass::Focused)
                }
                LightningPseudoClass::Custom { name } if name.as_ref() == "focused" => {
                    current_step.pseudo_classes.push(PseudoClass::Focused);
                }
                _ => {}
            },
            Component::Combinator(comb) => {
                current_step.combinator = Some(*comb);
                steps.push(current_step);
                current_step = SelectorStep {
                    tag: None,
                    id: None,
                    classes: Vec::new(),
                    pseudo_classes: Vec::new(),
                    combinator: None,
                };
            }
            _ => {}
        }
    }
    steps.push(current_step);

    CompiledSelector { steps }
}

fn matches_selector(selector: &CompiledSelector, query: &ElementQuery) -> bool {
    if selector.steps.is_empty() {
        return false;
    }

    let first_step = &selector.steps[0];
    if !step_matches(first_step, query) {
        return false;
    }

    if selector.steps.len() == 1 {
        return true;
    }

    let mut current_query = query.parent();
    let mut step_idx = 1;

    while step_idx < selector.steps.len() {
        let prev_comb = selector.steps[step_idx - 1]
            .combinator
            .unwrap_or(Combinator::Descendant);
        let step = &selector.steps[step_idx];

        match prev_comb {
            Combinator::Child => {
                let Some(q) = current_query else {
                    return false;
                };
                if !step_matches(step, q) {
                    return false;
                }
                current_query = q.parent();
                step_idx += 1;
            }
            Combinator::Descendant => {
                let mut matched = false;
                while let Some(q) = current_query {
                    if step_matches(step, q) {
                        matched = true;
                        current_query = q.parent();
                        step_idx += 1;
                        break;
                    }
                    current_query = q.parent();
                }
                if !matched {
                    return false;
                }
            }
            _ => {
                return false;
            }
        }
    }

    true
}

fn step_matches(step: &SelectorStep, query: &ElementQuery) -> bool {
    if let Some(tag) = &step.tag
        && tag != "*"
        && tag != &query.tag().to_lowercase()
    {
        return false;
    }

    if let Some(id) = &step.id {
        match query.id() {
            Some(qid) if qid.as_str() == id => {}
            _ => return false,
        }
    }

    for class in &step.classes {
        if !query.classes().iter().any(|c| c.as_str() == class) {
            return false;
        }
    }

    for pseudo in &step.pseudo_classes {
        if !query.pseudo_classes().contains(pseudo) {
            return false;
        }
    }

    true
}

fn parse_declarations(declarations: &DeclarationBlock) -> ComputedStyle {
    let mut style = ComputedStyle::default();

    for prop in declarations
        .declarations
        .iter()
        .chain(declarations.important_declarations.iter())
    {
        match prop {
            Property::BackgroundColor(color) => {
                if let Some(c) = convert_color(color) {
                    style.set_background(DrawingColor::Solid(c));
                }
            }
            Property::Color(color) => {
                if let Some(c) = convert_color(color) {
                    style.set_color(DrawingColor::Solid(c));
                }
            }
            Property::AccentColor(color) => {
                let s = color
                    .to_css_string(PrinterOptions::default())
                    .unwrap_or_default();
                if let Ok(c) = DrawingColor::parse(s.trim()) {
                    style.set_accent_color(c);
                }
            }
            Property::Custom(custom) => {
                let name = custom.name.as_ref();
                if name == "accent-color" || name == "progress-color" || name == "fill-color" {
                    for item in &custom.value.0 {
                        if let lightningcss::properties::custom::TokenOrValue::Color(c) = item
                            && let Some(col) = convert_color(c)
                        {
                            style.set_accent_color(DrawingColor::Solid(col));
                        }
                    }
                }
            }
            Property::FontFamily(families) => {
                if let Some(first) = families.first() {
                    let name = first
                        .to_css_string(PrinterOptions::default())
                        .unwrap_or_default();
                    let trimmed = name.trim_matches('"').trim_matches('\'').to_string();
                    style.set_font_family(FontFamily::new(trimmed));
                }
            }
            Property::FontSize(size) => match size {
                LightningFontSize::Length(l) => {
                    let px = parse_length_str(
                        &l.to_css_string(PrinterOptions::default())
                            .unwrap_or_default(),
                    );
                    style.set_font_size(FontSize::new(px));
                }
                LightningFontSize::Absolute(abs) => {
                    let px = match abs {
                        AbsoluteFontSize::XXSmall => 9.0,
                        AbsoluteFontSize::XSmall => 10.0,
                        AbsoluteFontSize::Small => 12.0,
                        AbsoluteFontSize::Medium => 14.0,
                        AbsoluteFontSize::Large => 18.0,
                        AbsoluteFontSize::XLarge => 24.0,
                        AbsoluteFontSize::XXLarge => 32.0,
                        AbsoluteFontSize::XXXLarge => 48.0,
                    };
                    style.set_font_size(FontSize::new(px));
                }
                _ => {}
            },
            Property::Border(border) => {
                apply_border(&mut style, border);
            }
            Property::BorderRadius(radius, _) => {
                let px = parse_length_str(
                    &radius
                        .top_left
                        .0
                        .to_css_string(PrinterOptions::default())
                        .unwrap_or_default(),
                );
                style.set_border_radius(BorderRadius::new(px));
            }
            Property::BorderWidth(width) => {
                let px = parse_length_str(
                    &width
                        .top
                        .to_css_string(PrinterOptions::default())
                        .unwrap_or_default(),
                );
                style.set_border_size(BorderSize::new(px));
            }
            Property::BorderColor(color) => {
                if let Some(c) = convert_color(&color.top) {
                    style.set_border_color(DrawingColor::Solid(c));
                }
            }
            Property::Padding(padding) => {
                let top = length_to_f64(&padding.top);
                let right = length_to_f64(&padding.right);
                let bottom = length_to_f64(&padding.bottom);
                let left = length_to_f64(&padding.left);
                style.set_padding(BoxMargin::new(top, bottom, left, right));
            }
            Property::Margin(margin) => {
                let top = length_to_f64(&margin.top);
                let right = length_to_f64(&margin.right);
                let bottom = length_to_f64(&margin.bottom);
                let left = length_to_f64(&margin.left);
                style.set_margin(BoxMargin::new(top, bottom, left, right));
            }
            Property::Gap(gap) => {
                let row_str = gap
                    .row
                    .to_css_string(PrinterOptions::default())
                    .unwrap_or_default();
                let row = parse_length_str(&row_str);
                style.set_gap(Gap::new(row as f64));
            }
            Property::FlexDirection(dir, _) => {
                let s = dir
                    .to_css_string(PrinterOptions::default())
                    .unwrap_or_default();
                let fd = match s.as_str() {
                    "column" => Some(FlexDirection::Column),
                    "row" => Some(FlexDirection::Row),
                    _ => None,
                };
                if let Some(d) = fd {
                    style.set_flex_direction(d);
                }
            }
            Property::JustifyContent(jc, _) => {
                let s = jc
                    .to_css_string(PrinterOptions::default())
                    .unwrap_or_default();
                let val = if s.contains("space-between") {
                    JustifyContent::SpaceBetween
                } else if s.contains("space-around") {
                    JustifyContent::SpaceAround
                } else if s.contains("space-evenly") {
                    JustifyContent::SpaceEvenly
                } else if s.contains("center") {
                    JustifyContent::Center
                } else if s.contains("end") || s.contains("flex-end") {
                    JustifyContent::End
                } else {
                    JustifyContent::Start
                };
                style.set_justify_content(val);
            }
            Property::AlignItems(ai, _) => {
                let s = ai
                    .to_css_string(PrinterOptions::default())
                    .unwrap_or_default();
                let val = if s.contains("center") {
                    AlignItems::Center
                } else if s.contains("end") || s.contains("flex-end") {
                    AlignItems::End
                } else if s.contains("stretch") {
                    AlignItems::Stretch
                } else {
                    AlignItems::Start
                };
                style.set_align_items(val);
            }
            Property::Position(pos) => {
                let val = match pos {
                    lightningcss::properties::position::Position::Absolute => {
                        PositionType::Absolute
                    }
                    _ => PositionType::Relative,
                };
                style.set_position(val);
            }
            Property::Width(size) => {
                let s = size
                    .to_css_string(PrinterOptions::default())
                    .unwrap_or_default();
                if let Some(w) = parse_size_str(&s) {
                    style.set_width(w);
                }
            }
            Property::Height(size) => {
                let s = size
                    .to_css_string(PrinterOptions::default())
                    .unwrap_or_default();
                if let Some(h) = parse_size_str(&s) {
                    style.set_height(h);
                }
            }
            Property::MinWidth(size) => {
                let s = size
                    .to_css_string(PrinterOptions::default())
                    .unwrap_or_default();
                if let Some(mw) = parse_size_str(&s) {
                    style.set_min_width(mw);
                }
            }
            Property::MaxWidth(size) => {
                let s = size
                    .to_css_string(PrinterOptions::default())
                    .unwrap_or_default();
                if let Some(mw) = parse_size_str(&s) {
                    style.set_max_width(mw);
                }
            }
            Property::MinHeight(size) => {
                let s = size
                    .to_css_string(PrinterOptions::default())
                    .unwrap_or_default();
                if let Some(mh) = parse_size_str(&s) {
                    style.set_min_height(mh);
                }
            }
            Property::MaxHeight(size) => {
                let s = size
                    .to_css_string(PrinterOptions::default())
                    .unwrap_or_default();
                if let Some(mh) = parse_size_str(&s) {
                    style.set_max_height(mh);
                }
            }
            Property::Background(bgs) => {
                let s = bgs
                    .to_css_string(PrinterOptions::default())
                    .unwrap_or_default();
                if let Ok(c) = DrawingColor::parse(s.trim()) {
                    style.set_background(c);
                } else if let Some(first) = bgs.first()
                    && let Some(c) = convert_color(&first.color)
                {
                    style.set_background(DrawingColor::Solid(c));
                }
            }
            Property::FlexGrow(fg, _) => {
                if let Ok(val) = FlexGrow::new(*fg) {
                    style.set_flex_grow(val);
                }
            }
            Property::FlexShrink(fs, _) => {
                if let Ok(val) = FlexShrink::new(*fs) {
                    style.set_flex_shrink(val);
                }
            }
            Property::FlexBasis(fb, _) => {
                let s = fb
                    .to_css_string(PrinterOptions::default())
                    .unwrap_or_default();
                if let Some(val) = parse_size_str(&s) {
                    style.set_flex_basis(val);
                }
            }
            Property::Flex(flex, _) => {
                if let Ok(val) = FlexGrow::new(flex.grow) {
                    style.set_flex_grow(val);
                }
                if let Ok(val) = FlexShrink::new(flex.shrink) {
                    style.set_flex_shrink(val);
                }
                let s = flex
                    .basis
                    .to_css_string(PrinterOptions::default())
                    .unwrap_or_default();
                if let Some(val) = parse_size_str(&s) {
                    style.set_flex_basis(val);
                }
            }
            Property::AlignSelf(as_, _) => {
                let s = as_
                    .to_css_string(PrinterOptions::default())
                    .unwrap_or_default();
                let val = if s.contains("center") {
                    AlignItems::Center
                } else if s.contains("end") || s.contains("flex-end") {
                    AlignItems::End
                } else if s.contains("stretch") {
                    AlignItems::Stretch
                } else {
                    AlignItems::Start
                };
                style.set_align_self(val);
            }
            Property::PaddingTop(len) => {
                let top = length_to_f64(len);
                let current = style.padding().cloned().unwrap_or_default();
                style.set_padding(BoxMargin::new(
                    top,
                    current.bottom(),
                    current.left(),
                    current.right(),
                ));
            }
            Property::PaddingRight(len) => {
                let right = length_to_f64(len);
                let current = style.padding().cloned().unwrap_or_default();
                style.set_padding(BoxMargin::new(
                    current.top(),
                    current.bottom(),
                    current.left(),
                    right,
                ));
            }
            Property::PaddingBottom(len) => {
                let bottom = length_to_f64(len);
                let current = style.padding().cloned().unwrap_or_default();
                style.set_padding(BoxMargin::new(
                    current.top(),
                    bottom,
                    current.left(),
                    current.right(),
                ));
            }
            Property::PaddingLeft(len) => {
                let left = length_to_f64(len);
                let current = style.padding().cloned().unwrap_or_default();
                style.set_padding(BoxMargin::new(
                    current.top(),
                    current.bottom(),
                    left,
                    current.right(),
                ));
            }
            Property::MarginTop(len) => {
                let top = length_to_f64(len);
                let current = style.margin().cloned().unwrap_or_default();
                style.set_margin(BoxMargin::new(
                    top,
                    current.bottom(),
                    current.left(),
                    current.right(),
                ));
            }
            Property::MarginRight(len) => {
                let right = length_to_f64(len);
                let current = style.margin().cloned().unwrap_or_default();
                style.set_margin(BoxMargin::new(
                    current.top(),
                    current.bottom(),
                    current.left(),
                    right,
                ));
            }
            Property::MarginBottom(len) => {
                let bottom = length_to_f64(len);
                let current = style.margin().cloned().unwrap_or_default();
                style.set_margin(BoxMargin::new(
                    current.top(),
                    bottom,
                    current.left(),
                    current.right(),
                ));
            }
            Property::MarginLeft(len) => {
                let left = length_to_f64(len);
                let current = style.margin().cloned().unwrap_or_default();
                style.set_margin(BoxMargin::new(
                    current.top(),
                    current.bottom(),
                    left,
                    current.right(),
                ));
            }
            Property::Opacity(op) => {
                let s = op
                    .to_css_string(PrinterOptions::default())
                    .unwrap_or_default();
                if let Ok(v) = s.parse::<f32>()
                    && let Ok(val) = Opacity::new(v)
                {
                    style.set_opacity(val);
                }
            }
            _ => {}
        }
    }

    style
}

fn parse_size_str(s: &str) -> Option<CssLength> {
    let trimmed = s.trim();
    if trimmed == "auto" {
        Some(CssLength::Auto)
    } else if let Some(pct) = trimmed.strip_suffix('%') {
        pct.trim()
            .parse::<f32>()
            .ok()
            .and_then(|v| CssLength::percent(v).ok())
    } else if trimmed == "none" {
        None
    } else {
        let px = parse_length_str(trimmed);
        CssLength::px(px).ok()
    }
}

fn apply_border(style: &mut ComputedStyle, border: &Border) {
    let px = parse_length_str(
        &border
            .width
            .to_css_string(PrinterOptions::default())
            .unwrap_or_default(),
    );
    style.set_border_size(BorderSize::new(px));
    if let Some(c) = convert_color(&border.color) {
        style.set_border_color(DrawingColor::Solid(c));
    }
}

fn parse_length_str(s: &str) -> f32 {
    let s = s.trim();
    if let Some(num) = s.strip_suffix("px") {
        num.trim().parse::<f32>().unwrap_or(0.0)
    } else if let Some(num) = s.strip_suffix("rem") {
        num.trim().parse::<f32>().unwrap_or(0.0) * 16.0
    } else if let Some(num) = s.strip_suffix("em") {
        num.trim().parse::<f32>().unwrap_or(0.0) * 16.0
    } else {
        s.parse::<f32>().unwrap_or(0.0)
    }
}

fn convert_color(color: &LightningCssColor) -> Option<Color> {
    match color {
        LightningCssColor::RGBA(rgba) => {
            Some(Color::new(rgba.red, rgba.green, rgba.blue, rgba.alpha))
        }
        _ => {
            let raw = color.to_css_string(PrinterOptions::default()).ok()?;
            if let Ok(DrawingColor::Solid(c)) = DrawingColor::parse(&raw) {
                Some(c)
            } else {
                None
            }
        }
    }
}

fn length_to_f64(len: &LengthPercentageOrAuto) -> f64 {
    let s = len
        .to_css_string(PrinterOptions::default())
        .unwrap_or_default();
    parse_length_str(&s) as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::styling::domain::ClassName;

    #[test]
    fn test_parse_basic_css_rules() {
        let parser = LightningCssAdapter::new();
        let css = r#"
            bar {
                background-color: #1a1b26;
                padding: 4px 8px;
                gap: 6px;
            }
            .workspace-btn {
                background-color: #3b4261;
                border-radius: 4px;
                font-size: 14px;
            }
            .workspace-btn:focus, .workspace-btn:hover {
                background-color: #7aa2f7;
            }
            #hour-main {
                color: #c0caf5;
            }
            progress {
                background-color: #24283b;
                border-radius: 6px;
                accent-color: #bb9af7;
            }
        "#;

        let parsed = parser
            .parse_stylesheet(StyleSheetName::new("test").unwrap(), css)
            .expect("Failed to parse stylesheet");

        // Test matching .workspace-btn
        let ws_class = ClassName::new("workspace-btn").unwrap();
        let query_ws = ElementQuery::new("flex", None, std::slice::from_ref(&ws_class), &[], None);
        let style_ws = parsed.resolve_style(&query_ws);
        assert_eq!(style_ws.border_radius().unwrap().value(), 4.0);
        assert_eq!(style_ws.font_size().unwrap().value(), 14.0);

        // Test matching .workspace-btn:hover
        let query_ws_hover = ElementQuery::new(
            "flex",
            None,
            std::slice::from_ref(&ws_class),
            &[PseudoClass::Hover],
            None,
        );
        let style_ws_hover = parsed.resolve_style(&query_ws_hover);
        if let Some(DrawingColor::Solid(c)) = style_ws_hover.background() {
            assert_eq!(*c, Color::new(122, 162, 247, 255));
        } else {
            panic!("Expected background color #7aa2f7");
        }

        // Test matching #hour-main
        let hour_id = crate::features::styling::domain::ElementId::new("hour-main").unwrap();
        let query_hour = ElementQuery::new("text", Some(&hour_id), &[], &[], None);
        let style_hour = parsed.resolve_style(&query_hour);
        if let Some(DrawingColor::Solid(c)) = style_hour.color() {
            assert_eq!(*c, Color::new(192, 202, 245, 255));
        } else {
            panic!("Expected text color #c0caf5");
        }

        // Test matching progress
        let query_progress = ElementQuery::new("progress", None, &[], &[], None);
        let style_prog = parsed.resolve_style(&query_progress);
        assert_eq!(style_prog.border_radius().unwrap().value(), 6.0);
        assert!(style_prog.accent_color().is_some());
    }

    #[test]
    fn test_descendant_combinator() {
        let parser = LightningCssAdapter::new();
        let css = r#"
            bar .item {
                color: #ffffff;
            }
        "#;
        let parsed = parser
            .parse_stylesheet(StyleSheetName::new("test").unwrap(), css)
            .unwrap();

        let bar_parent = ElementQuery::new("bar", None, &[], &[], None);

        let item_class = ClassName::new("item").unwrap();
        let item_query = ElementQuery::new(
            "text",
            None,
            std::slice::from_ref(&item_class),
            &[],
            Some(&bar_parent),
        );

        let style = parsed.resolve_style(&item_query);
        assert!(style.color().is_some());
    }

    #[test]
    fn test_arbitrary_style_name_and_progress_rendering() {
        use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
        use crate::features::layout_engine::ports::LayoutEnginePort;
        use crate::features::styling::domain::{ClassNameList, Orientation, ProgressValue};
        use crate::features::vdom::domain::{TextContent, VNode};
        use crate::shared::primitives::geometry::Position;
        use crate::shared::rendering::ports::canvas::MockCanvas;

        let parser = LightningCssAdapter::new();
        let theme_css = r#"
            .clock-label {
                color: #ff5555;
                font-size: 16px;
            }
            progress.battery {
                background-color: #282a36;
                accent-color: #50fa7b;
                border-radius: 4px;
            }
        "#;

        let parsed = parser
            .parse_stylesheet(StyleSheetName::new("random").unwrap(), theme_css)
            .unwrap();
        let resolver =
            crate::features::styling::adapters::fs_loader::CompositeStyleResolver::new(vec![
                parsed,
            ]);

        // 1. Clock requesting random style
        let clock_node = VNode::new_text(
            TextContent::new("12:00".to_string()),
            Some(ClassNameList::parse("clock-label").unwrap()),
            None,
            None,
            None,
            None,
        );
        let styled_clock = clock_node.resolve_styles(&resolver, None);
        assert_eq!(styled_clock.style().font_size().unwrap().value(), 16.0);

        // 2. Progress bar horizontal & vertical rendering
        let h_prog = VNode::new_progress(
            ProgressValue::new(0.6).unwrap(),
            Orientation::Horizontal,
            Some(ClassNameList::parse("battery").unwrap()),
            None,
            None,
            None,
            None,
        );
        let styled_h = h_prog.resolve_styles(&resolver, None);
        let mut engine = TaffyLayoutAdapter::new();

        struct DummyMeasurer;
        impl crate::features::layout_engine::domain::TextMeasurer for DummyMeasurer {
            fn measure(
                &mut self,
                text: &str,
                _f: Option<&crate::shared::config::domain::FontFamily>,
                _s: Option<crate::shared::config::domain::FontSize>,
            ) -> crate::shared::primitives::geometry::Size {
                crate::shared::primitives::geometry::Size::new(text.len() as u32 * 8, 16)
            }
        }

        let render_h = engine
            .calculate_layout(styled_h, &mut DummyMeasurer, Position::new(0, 0))
            .unwrap();
        let mut mock_canvas = MockCanvas::new();
        mock_canvas.expect_draw_rect().times(2).return_const(());
        render_h.render_to_canvas(&mut mock_canvas);
    }

    #[test]
    fn test_width_height_parsing_and_layout() {
        use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
        use crate::features::layout_engine::ports::LayoutEnginePort;
        use crate::features::styling::domain::ClassNameList;
        use crate::features::vdom::domain::VNode;
        use crate::shared::primitives::geometry::{Position, Size};

        let parser = LightningCssAdapter::new();
        let css = r#"
            .icon {
                width: 20px;
                height: 20px;
            }
        "#;
        let parsed = parser
            .parse_stylesheet(StyleSheetName::new("applet").unwrap(), css)
            .unwrap();
        let resolver =
            crate::features::styling::adapters::fs_loader::CompositeStyleResolver::new(vec![
                parsed,
            ]);

        let img_node = VNode::new_image(
            vec![0; 400 * 4],
            Size::new(48, 48),
            Some(ClassNameList::parse("icon").unwrap()),
            None,
            None,
        );

        let styled_img = img_node.resolve_styles(&resolver, None);
        assert_eq!(
            styled_img.style().width(),
            Some(crate::features::styling::domain::CssLength::Px(20.0))
        );
        assert_eq!(
            styled_img.style().height(),
            Some(crate::features::styling::domain::CssLength::Px(20.0))
        );

        let mut engine = TaffyLayoutAdapter::new();
        struct DummyMeasurer;
        impl crate::features::layout_engine::domain::TextMeasurer for DummyMeasurer {
            fn measure(
                &mut self,
                _text: &str,
                _f: Option<&crate::shared::config::domain::FontFamily>,
                _s: Option<crate::shared::config::domain::FontSize>,
            ) -> crate::shared::primitives::geometry::Size {
                crate::shared::primitives::geometry::Size::new(10, 10)
            }
        }

        let render_node = engine
            .calculate_layout(styled_img, &mut DummyMeasurer, Position::new(0, 0))
            .unwrap();

        assert_eq!(render_node.rect().width(), 20);
        assert_eq!(render_node.rect().height(), 20);
    }

    #[test]
    fn test_advanced_css_properties_parsing_and_layout() {
        let parser = LightningCssAdapter::new();
        let css = r#"
            .box {
                background: #1e1e2e;
                flex-grow: 1;
                flex-shrink: 0;
                align-self: center;
                padding-left: 12px;
                padding-right: 8px;
                border: 2px solid #7aa2f7;
            }
        "#;
        let parsed = parser
            .parse_stylesheet(StyleSheetName::new("box").unwrap(), css)
            .unwrap();

        let class = crate::features::styling::domain::ClassName::new("box").unwrap();
        let query = ElementQuery::new("flex", None, std::slice::from_ref(&class), &[], None);

        let style = parsed.resolve_style(&query);
        assert!(style.background().is_some());
        assert_eq!(style.flex_grow().unwrap().value(), 1.0);
        assert_eq!(style.flex_shrink().unwrap().value(), 0.0);
        assert_eq!(style.align_self(), Some(AlignItems::Center));
        assert_eq!(style.padding().unwrap().left(), 12.0);
        assert_eq!(style.padding().unwrap().right(), 8.0);
        assert_eq!(style.border_size().unwrap().value(), 2.0);
    }
}
