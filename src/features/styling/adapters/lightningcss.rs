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
use parcel_selectors::parser::NthType;

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
    negations: Vec<CompiledSelector>,
    combinator: Option<Combinator>,
}

pub struct LightningCssAdapter;

impl LightningCssAdapter {
    #[must_use]
    pub const fn new() -> Self {
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
            id = ?query.id().map(super::super::domain::ElementId::as_str),
            classes = ?query.classes().iter().map(super::super::domain::ClassName::as_str).collect::<Vec<_>>(),
            has_bg = result.background().is_some(),
            has_color = result.color().is_some(),
            "Resolved style for query"
        );

        result
    }
}

fn compile_nth_data(nth_data: &parcel_selectors::parser::NthSelectorData, step: &mut SelectorStep) {
    match nth_data.ty {
        NthType::Child => {
            if nth_data.is_function() {
                step.pseudo_classes.push(PseudoClass::NthChild {
                    a: nth_data.a,
                    b: nth_data.b,
                });
            } else {
                step.pseudo_classes.push(PseudoClass::FirstChild);
            }
        }
        NthType::LastChild => {
            if nth_data.is_function() {
                step.pseudo_classes.push(PseudoClass::NthLastChild {
                    a: nth_data.a,
                    b: nth_data.b,
                });
            } else {
                step.pseudo_classes.push(PseudoClass::LastChild);
            }
        }
        NthType::OnlyChild => {
            step.pseudo_classes.push(PseudoClass::OnlyChild);
        }
        _ => {}
    }
}

fn compile_pseudo_class(pseudo: &LightningPseudoClass, step: &mut SelectorStep) {
    match pseudo {
        LightningPseudoClass::Hover => step.pseudo_classes.push(PseudoClass::Hover),
        LightningPseudoClass::Active => step.pseudo_classes.push(PseudoClass::Active),
        LightningPseudoClass::Focus | LightningPseudoClass::FocusVisible => {
            step.pseudo_classes.push(PseudoClass::Focused);
        }
        LightningPseudoClass::Custom { name } if name.as_ref() == "focused" => {
            step.pseudo_classes.push(PseudoClass::Focused);
        }
        _ => {}
    }
}

fn compile_selector(selector: &Selector) -> CompiledSelector {
    let mut steps = Vec::new();
    let mut current_step = SelectorStep {
        tag: None,
        id: None,
        classes: Vec::new(),
        pseudo_classes: Vec::new(),
        negations: Vec::new(),
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
            Component::Nth(nth_data) => compile_nth_data(nth_data, &mut current_step),
            Component::NthOf(nth_of_data) => {
                compile_nth_data(nth_of_data.nth_data(), &mut current_step);
            }
            Component::Empty => {
                current_step.pseudo_classes.push(PseudoClass::Empty);
            }
            Component::Negation(selectors) => {
                for inner_sel in selectors.as_ref() {
                    current_step.negations.push(compile_selector(inner_sel));
                }
            }
            Component::NonTSPseudoClass(pseudo) => {
                compile_pseudo_class(pseudo, &mut current_step);
            }
            Component::Combinator(comb) => {
                current_step.combinator = Some(*comb);
                steps.push(current_step);
                current_step = SelectorStep {
                    tag: None,
                    id: None,
                    classes: Vec::new(),
                    pseudo_classes: Vec::new(),
                    negations: Vec::new(),
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
    let Some(first_step) = selector.steps.first() else {
        return false;
    };
    if !step_matches(first_step, query) {
        return false;
    }

    if selector.steps.len() == 1 {
        return true;
    }

    let mut current_query = query.parent();
    let mut step_idx = 1usize;

    while step_idx < selector.steps.len() {
        let prev_comb = selector
            .steps
            .get(step_idx.saturating_sub(1))
            .and_then(|s| s.combinator)
            .unwrap_or(Combinator::Descendant);
        let Some(step) = selector.steps.get(step_idx) else {
            return false;
        };

        match prev_comb {
            Combinator::Child => {
                let Some(q) = current_query else {
                    return false;
                };
                if !step_matches(step, q) {
                    return false;
                }
                current_query = q.parent();
                step_idx = step_idx.saturating_add(1);
            }
            Combinator::Descendant => {
                let mut matched = false;
                while let Some(q) = current_query {
                    if step_matches(step, q) {
                        matched = true;
                        current_query = q.parent();
                        step_idx = step_idx.saturating_add(1);
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
        if !pseudo_matches(pseudo, query) {
            return false;
        }
    }

    for not_sel in &step.negations {
        if matches_selector(not_sel, query) {
            return false;
        }
    }

    true
}

fn pseudo_matches(pseudo: &PseudoClass, query: &ElementQuery) -> bool {
    match pseudo {
        PseudoClass::Hover | PseudoClass::Active | PseudoClass::Focused => {
            query.pseudo_classes().contains(pseudo)
        }
        PseudoClass::FirstChild => query.child_index() == 0,
        PseudoClass::LastChild => {
            query.total_children() > 0
                && query.child_index().saturating_add(1) == query.total_children()
        }
        PseudoClass::OnlyChild => query.total_children() == 1,
        PseudoClass::NthChild { a, b } => {
            nth_matches(*a, *b, query.child_index().saturating_add(1))
        }
        PseudoClass::NthLastChild { a, b } => {
            let from_end = query.total_children().saturating_sub(query.child_index());
            nth_matches(*a, *b, from_end)
        }
        PseudoClass::Empty => query.is_empty(),
    }
}

fn nth_matches(a: i32, b: i32, index_1based: usize) -> bool {
    let Ok(n_pos) = i32::try_from(index_1based) else {
        return false;
    };
    if a == 0 {
        return n_pos == b;
    }
    let Some(diff) = n_pos.checked_sub(b) else {
        return false;
    };
    if a > 0 {
        diff >= 0 && diff.checked_rem(a) == Some(0)
    } else {
        diff <= 0 && diff.checked_rem(a) == Some(0)
    }
}

fn apply_color_and_font(style: &mut ComputedStyle, prop: &Property) -> bool {
    match prop {
        Property::BackgroundColor(color) => {
            if let Some(c) = convert_color(color) {
                style.set_background(DrawingColor::Solid(c));
            }
            true
        }
        Property::Color(color) => {
            if let Some(c) = convert_color(color) {
                style.set_color(DrawingColor::Solid(c));
            }
            true
        }
        Property::AccentColor(color) => {
            let s = color
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Ok(c) = DrawingColor::parse(s.trim()) {
                style.set_accent_color(c);
            }
            true
        }
        Property::Custom(custom) => {
            apply_custom_or_unparsed_property(style, custom.name.as_ref(), prop);
            true
        }
        Property::Unparsed(unparsed) => {
            apply_custom_or_unparsed_property(style, unparsed.property_id.name(), prop);
            true
        }
        Property::FontFamily(families) => {
            if let Some(first) = families.first() {
                let name = first
                    .to_css_string(PrinterOptions::default())
                    .unwrap_or_default();
                let trimmed = name.trim_matches('"').trim_matches('\'').to_string();
                style.set_font_family(FontFamily::new(trimmed));
            }
            true
        }
        Property::FontSize(size) => {
            apply_font_size(style, size);
            true
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
            true
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
            true
        }
        _ => false,
    }
}

fn apply_custom_or_unparsed_property(style: &mut ComputedStyle, name: &str, prop: &Property) {
    if (name == "accent-color" || name == "progress-color" || name == "fill-color")
        && let Ok(full) = prop.to_css_string(false, PrinterOptions::default())
    {
        let val = full.split_once(':').map_or(&*full, |(_, v)| v);
        if let Ok(c) = DrawingColor::parse(val.trim()) {
            style.set_accent_color(c);
        }
    } else if (name == "border-color" || name == "border")
        && let Ok(full) = prop.to_css_string(false, PrinterOptions::default())
    {
        let val = full.split_once(':').map_or(&*full, |(_, v)| v);
        if let Ok(c) = DrawingColor::parse(val.trim()) {
            style.set_border_color(c);
        }
    }
}

fn apply_font_size(style: &mut ComputedStyle, size: &LightningFontSize) {
    match size {
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
        LightningFontSize::Relative(_) => {}
    }
}

fn apply_border_properties(style: &mut ComputedStyle, prop: &Property) -> bool {
    match prop {
        Property::Border(border) => {
            apply_border(style, border);
            true
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
            true
        }
        Property::BorderWidth(width) => {
            let px = parse_length_str(
                &width
                    .top
                    .to_css_string(PrinterOptions::default())
                    .unwrap_or_default(),
            );
            style.set_border_size(BorderSize::new(px));
            true
        }
        Property::BorderColor(color) => {
            let s = color
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Ok(c) = DrawingColor::parse(s.trim()) {
                style.set_border_color(c);
            } else if let Some(c) = convert_color(&color.top) {
                style.set_border_color(DrawingColor::Solid(c));
            }
            true
        }
        _ => false,
    }
}

fn apply_padding_properties(style: &mut ComputedStyle, prop: &Property) -> bool {
    match prop {
        Property::Padding(padding) => {
            let top = length_to_f64(&padding.top);
            let right = length_to_f64(&padding.right);
            let bottom = length_to_f64(&padding.bottom);
            let left = length_to_f64(&padding.left);
            style.set_padding(BoxMargin::new(top, bottom, left, right));
            true
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
            true
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
            true
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
            true
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
            true
        }
        _ => false,
    }
}

fn apply_margin_properties(style: &mut ComputedStyle, prop: &Property) -> bool {
    match prop {
        Property::Margin(margin) => {
            let top = length_to_f64(&margin.top);
            let right = length_to_f64(&margin.right);
            let bottom = length_to_f64(&margin.bottom);
            let left = length_to_f64(&margin.left);
            style.set_margin(BoxMargin::new(top, bottom, left, right));
            true
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
            true
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
            true
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
            true
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
            true
        }
        _ => false,
    }
}

fn apply_sizing_properties(style: &mut ComputedStyle, prop: &Property) -> bool {
    match prop {
        Property::Width(size) => {
            let s = size
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Some(w) = parse_size_str(&s) {
                style.set_width(w);
            }
            true
        }
        Property::Height(size) => {
            let s = size
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Some(h) = parse_size_str(&s) {
                style.set_height(h);
            }
            true
        }
        Property::MinWidth(size) => {
            let s = size
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Some(mw) = parse_size_str(&s) {
                style.set_min_width(mw);
            }
            true
        }
        Property::MaxWidth(size) => {
            let s = size
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Some(mw) = parse_size_str(&s) {
                style.set_max_width(mw);
            }
            true
        }
        Property::MinHeight(size) => {
            let s = size
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Some(mh) = parse_size_str(&s) {
                style.set_min_height(mh);
            }
            true
        }
        Property::MaxHeight(size) => {
            let s = size
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Some(mh) = parse_size_str(&s) {
                style.set_max_height(mh);
            }
            true
        }
        _ => false,
    }
}

fn apply_flex_container_properties(style: &mut ComputedStyle, prop: &Property) -> bool {
    match prop {
        Property::Gap(gap) => {
            let row_str = gap
                .row
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            let row = parse_length_str(&row_str);
            style.set_gap(Gap::new(f64::from(row)));
            true
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
            true
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
            true
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
            true
        }
        Property::Position(pos) => {
            let val = match pos {
                lightningcss::properties::position::Position::Absolute => PositionType::Absolute,
                _ => PositionType::Relative,
            };
            style.set_position(val);
            true
        }
        _ => false,
    }
}

fn apply_flex_item_properties(style: &mut ComputedStyle, prop: &Property) -> bool {
    match prop {
        Property::FlexGrow(fg, _) => {
            if let Ok(val) = FlexGrow::new(*fg) {
                style.set_flex_grow(val);
            }
            true
        }
        Property::FlexShrink(fs, _) => {
            if let Ok(val) = FlexShrink::new(*fs) {
                style.set_flex_shrink(val);
            }
            true
        }
        Property::FlexBasis(fb, _) => {
            let s = fb
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Some(val) = parse_size_str(&s) {
                style.set_flex_basis(val);
            }
            true
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
            true
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
            true
        }
        _ => false,
    }
}

fn parse_declarations(declarations: &DeclarationBlock) -> ComputedStyle {
    let mut style = ComputedStyle::default();

    for prop in declarations
        .declarations
        .iter()
        .chain(declarations.important_declarations.iter())
    {
        let _ = apply_color_and_font(&mut style, prop)
            || apply_border_properties(&mut style, prop)
            || apply_padding_properties(&mut style, prop)
            || apply_margin_properties(&mut style, prop)
            || apply_sizing_properties(&mut style, prop)
            || apply_flex_container_properties(&mut style, prop)
            || apply_flex_item_properties(&mut style, prop);
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
    let s = border
        .color
        .to_css_string(PrinterOptions::default())
        .unwrap_or_default();
    if let Ok(c) = DrawingColor::parse(s.trim()) {
        style.set_border_color(c);
    } else if let Some(c) = convert_color(&border.color) {
        style.set_border_color(DrawingColor::Solid(c));
    }
}

fn parse_length_str(s: &str) -> f32 {
    let s = s.trim();
    if let Some(num) = s.strip_suffix("px") {
        return num.trim().parse::<f32>().unwrap_or(0.0);
    }
    if let Some(num) = s.strip_suffix("rem") {
        return num.trim().parse::<f32>().unwrap_or(0.0) * 16.0;
    }
    if let Some(num) = s.strip_suffix("em") {
        return num.trim().parse::<f32>().unwrap_or(0.0) * 16.0;
    }
    s.parse::<f32>().unwrap_or(0.0)
}

fn convert_color(color: &LightningCssColor) -> Option<Color> {
    if let LightningCssColor::RGBA(rgba) = color {
        Some(Color::new(rgba.red, rgba.green, rgba.blue, rgba.alpha))
    } else {
        let raw = color.to_css_string(PrinterOptions::default()).ok()?;
        if let Ok(DrawingColor::Solid(c)) = DrawingColor::parse(&raw) {
            Some(c)
        } else {
            None
        }
    }
}

fn length_to_f64(len: &LengthPercentageOrAuto) -> f64 {
    let s = len
        .to_css_string(PrinterOptions::default())
        .unwrap_or_default();
    f64::from(parse_length_str(&s))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::styling::domain::ClassName;

    struct DummyMeasurer;
    impl crate::features::layout_engine::domain::TextMeasurer for DummyMeasurer {
        fn measure(
            &mut self,
            text: &str,
            _f: Option<&crate::shared::config::domain::FontFamily>,
            _s: Option<crate::shared::config::domain::FontSize>,
        ) -> crate::shared::primitives::geometry::Size {
            let text_len = u32::try_from(text.len()).unwrap_or(0);
            crate::shared::primitives::geometry::Size::new(text_len.saturating_mul(8), 16)
        }
    }

    struct FixedDummyMeasurer;
    impl crate::features::layout_engine::domain::TextMeasurer for FixedDummyMeasurer {
        fn measure(
            &mut self,
            _text: &str,
            _f: Option<&crate::shared::config::domain::FontFamily>,
            _s: Option<crate::shared::config::domain::FontSize>,
        ) -> crate::shared::primitives::geometry::Size {
            crate::shared::primitives::geometry::Size::new(10, 10)
        }
    }

    #[test]
    fn test_parse_basic_css_rules() {
        let parser = LightningCssAdapter::new();
        let css = r"
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
        ";

        let parsed = parser
            .parse_stylesheet(StyleSheetName::new("test").unwrap(), css)
            .expect("Failed to parse stylesheet");

        // Test matching .workspace-btn
        let ws_class = ClassName::new("workspace-btn").unwrap();
        let query_ws = ElementQuery::new("flex", None, std::slice::from_ref(&ws_class), &[], None);
        let style_ws = parsed.resolve_style(&query_ws);
        assert!((style_ws.border_radius().unwrap().value() - 4.0).abs() < f32::EPSILON);
        assert!((style_ws.font_size().unwrap().value() - 14.0).abs() < f32::EPSILON);

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
        assert!((style_prog.border_radius().unwrap().value() - 6.0).abs() < f32::EPSILON);
        assert!(style_prog.accent_color().is_some());
    }

    #[test]
    fn test_descendant_combinator() {
        let parser = LightningCssAdapter::new();
        let css = r"
            bar .item {
                color: #ffffff;
            }
        ";
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
        let theme_css = r"
            .clock-label {
                color: #ff5555;
                font-size: 16px;
            }
            progress.battery {
                background-color: #282a36;
                accent-color: #50fa7b;
                border-radius: 4px;
            }
        ";

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
        let styled_clock = clock_node.resolve_styles(&resolver, None, None);
        assert!((styled_clock.style().font_size().unwrap().value() - 16.0).abs() < f32::EPSILON);

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
        let styled_h = h_prog.resolve_styles(&resolver, None, None);
        let mut engine = TaffyLayoutAdapter::new();

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
        let css = r"
            .icon {
                width: 20px;
                height: 20px;
            }
        ";
        let parsed = parser
            .parse_stylesheet(StyleSheetName::new("systray").unwrap(), css)
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

        let styled_img = img_node.resolve_styles(&resolver, None, None);
        assert_eq!(
            styled_img.style().width(),
            Some(crate::features::styling::domain::CssLength::Px(20.0))
        );
        assert_eq!(
            styled_img.style().height(),
            Some(crate::features::styling::domain::CssLength::Px(20.0))
        );

        let mut engine = TaffyLayoutAdapter::new();

        let render_node = engine
            .calculate_layout(styled_img, &mut FixedDummyMeasurer, Position::new(0, 0))
            .unwrap();

        assert_eq!(render_node.rect().width(), 20);
        assert_eq!(render_node.rect().height(), 20);
    }

    #[test]
    fn test_advanced_css_properties_parsing_and_layout() {
        let parser = LightningCssAdapter::new();
        let css = r"
            .box {
                background: #1e1e2e;
                flex-grow: 1;
                flex-shrink: 0;
                align-self: center;
                padding-left: 12px;
                padding-right: 8px;
                border: 2px solid #7aa2f7;
            }
        ";
        let parsed = parser
            .parse_stylesheet(StyleSheetName::new("box").unwrap(), css)
            .unwrap();

        let class = crate::features::styling::domain::ClassName::new("box").unwrap();
        let query = ElementQuery::new("flex", None, std::slice::from_ref(&class), &[], None);

        let style = parsed.resolve_style(&query);
        assert!(style.background().is_some());
        assert!((style.flex_grow().unwrap().value() - 1.0).abs() < f32::EPSILON);
        assert!((style.flex_shrink().unwrap().value() - 0.0).abs() < f32::EPSILON);
        assert_eq!(style.align_self(), Some(AlignItems::Center));
        assert!((style.padding().unwrap().left() - 12.0).abs() < f64::EPSILON);
        assert!((style.padding().unwrap().right() - 8.0).abs() < f64::EPSILON);
        assert!((style.border_size().unwrap().value() - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_structural_pseudo_classes() {
        let parser = LightningCssAdapter::new();
        let css = r"
            .item:first-child {
                padding-left: 10px;
            }
            .item:last-child {
                padding-right: 15px;
            }
            .item:only-child {
                border-radius: 8px;
            }
            .item:nth-child(2n+1) {
                background-color: #111111;
            }
            .item:nth-child(2n) {
                background-color: #222222;
            }
            .item:empty {
                opacity: 0.5;
            }
            .item:not(.active) {
                color: #888888;
            }
        ";
        let parsed = parser
            .parse_stylesheet(StyleSheetName::new("test").unwrap(), css)
            .unwrap();

        let class_item = ClassName::new("item").unwrap();
        let class_active = ClassName::new("active").unwrap();

        // 1. First child in a list of 3 (index 0, total 3)
        let query_first =
            ElementQuery::new("flex", None, std::slice::from_ref(&class_item), &[], None)
                .with_structural_context(0, 3, false);
        let style_first = parsed.resolve_style(&query_first);
        assert_eq!(style_first.padding().map(BoxMargin::left), Some(10.0));
        assert_eq!(style_first.padding().map(BoxMargin::right), Some(0.0));
        assert!(style_first.background().is_some());
        // Odd: 1st is 2n+1 -> #111111
        if let Some(DrawingColor::Solid(c)) = style_first.background() {
            assert_eq!(*c, Color::new(17, 17, 17, 255));
        }

        // 2. Second child (index 1, total 3)
        let query_second =
            ElementQuery::new("flex", None, std::slice::from_ref(&class_item), &[], None)
                .with_structural_context(1, 3, false);
        let style_second = parsed.resolve_style(&query_second);
        assert!(style_second.padding().is_none());
        // Even: 2nd is 2n -> #222222
        if let Some(DrawingColor::Solid(c)) = style_second.background() {
            assert_eq!(*c, Color::new(34, 34, 34, 255));
        }

        // 3. Last child (index 2, total 3)
        let query_last =
            ElementQuery::new("flex", None, std::slice::from_ref(&class_item), &[], None)
                .with_structural_context(2, 3, false);
        let style_last = parsed.resolve_style(&query_last);
        assert_eq!(style_last.padding().map(BoxMargin::right), Some(15.0));

        // 4. Only child (index 0, total 1)
        let query_only =
            ElementQuery::new("flex", None, std::slice::from_ref(&class_item), &[], None)
                .with_structural_context(0, 1, false);
        let style_only = parsed.resolve_style(&query_only);
        assert_eq!(style_only.border_radius().map(|r| r.value()), Some(8.0));

        // 5. Empty
        let query_empty =
            ElementQuery::new("flex", None, std::slice::from_ref(&class_item), &[], None)
                .with_structural_context(0, 1, true);
        let style_empty = parsed.resolve_style(&query_empty);
        assert!((style_empty.opacity().unwrap().value() - 0.5).abs() < f32::EPSILON);

        // 6. Not active vs Active
        let query_not_active =
            ElementQuery::new("flex", None, std::slice::from_ref(&class_item), &[], None);
        let style_not_active = parsed.resolve_style(&query_not_active);
        assert!(style_not_active.color().is_some());

        let active_classes = [class_item.clone(), class_active];
        let query_active = ElementQuery::new("flex", None, &active_classes, &[], None);
        let style_active = parsed.resolve_style(&query_active);
        assert!(style_active.color().is_none());
    }

    #[test]
    fn test_gradient_border_color_resolution() {
        let parser = LightningCssAdapter::new();
        let css = r"
            bar {
                border-width: 2px;
                border-color: #565f89;
            }
            bar:focus {
                border-color: #7aa2f7 #bb9af7 45deg;
            }
        ";
        let parsed = parser
            .parse_stylesheet(StyleSheetName::new("test").unwrap(), css)
            .unwrap();

        // 1. Unfocused bar -> Solid #565f89
        let query_unfocused = ElementQuery::new("bar", None, &[], &[], None);
        let style_unfocused = parsed.resolve_style(&query_unfocused);
        assert_eq!(
            style_unfocused.border_color(),
            Some(&DrawingColor::Solid(Color::new(86, 95, 137, 255)))
        );

        // 2. Focused bar -> Gradient #7aa2f7 #bb9af7 45deg
        let query_focused = ElementQuery::new("bar", None, &[], &[PseudoClass::Focused], None);
        let style_focused = parsed.resolve_style(&query_focused);
        if let Some(DrawingColor::Gradient(colors, angle)) = style_focused.border_color() {
            assert_eq!(colors.len(), 2);
            assert_eq!(colors[0], Color::new(122, 162, 247, 255));
            assert_eq!(colors[1], Color::new(187, 154, 247, 255));
            assert!((angle - 45.0).abs() < f32::EPSILON);
        } else {
            panic!("Expected gradient border color on focused bar");
        }
    }
}
