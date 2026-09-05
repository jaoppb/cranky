use crate::features::styling::domain::{ComputedStyle, Opacity};
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::color::{Color, DrawingColor};
use lightningcss::properties::Property;
use lightningcss::properties::font::{AbsoluteFontSize, FontSize as LightningFontSize};
use lightningcss::stylesheet::PrinterOptions;
use lightningcss::traits::ToCss;
use lightningcss::values::color::CssColor as LightningCssColor;

pub fn apply_color_and_font(style: &mut ComputedStyle, prop: &Property) -> bool {
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

pub fn apply_custom_or_unparsed_property(
    style: &mut ComputedStyle,
    name: &str,
    prop: &Property,
) {
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

pub fn apply_font_size(style: &mut ComputedStyle, size: &LightningFontSize) {
    match size {
        LightningFontSize::Length(l) => {
            let px = super::box_model::parse_length_str(
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

#[must_use]
pub fn convert_color(color: &LightningCssColor) -> Option<Color> {
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
