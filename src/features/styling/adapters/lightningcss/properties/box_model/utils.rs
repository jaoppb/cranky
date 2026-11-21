use crate::features::styling::domain::CssLength;
use lightningcss::stylesheet::PrinterOptions;
use lightningcss::traits::ToCss;
use lightningcss::values::length::LengthPercentageOrAuto;

#[must_use]
pub(crate) fn parse_size_str(text: &str) -> Option<CssLength> {
    let trimmed = text.trim();
    if trimmed == "auto" {
        Some(CssLength::Auto)
    } else if let Some(pct) = trimmed.strip_suffix('%') {
        pct.trim()
            .parse::<f32>()
            .ok()
            .and_then(|percentage| CssLength::percent(percentage).ok())
    } else if trimmed == "none" {
        None
    } else {
        let px = parse_length_str(trimmed);
        CssLength::px(px).ok()
    }
}

#[must_use]
pub(crate) fn parse_length_str(text: &str) -> f32 {
    let text = text.trim();
    if let Some(num) = text.strip_suffix("px") {
        return num.trim().parse::<f32>().unwrap_or(0.0);
    }
    if let Some(num) = text.strip_suffix("rem") {
        return num.trim().parse::<f32>().unwrap_or(0.0) * 16.0;
    }
    if let Some(num) = text.strip_suffix("em") {
        return num.trim().parse::<f32>().unwrap_or(0.0) * 16.0;
    }
    text.parse::<f32>().unwrap_or(0.0)
}

#[must_use]
pub(crate) fn length_to_f64(len: &LengthPercentageOrAuto) -> f64 {
    let css_text = len
        .to_css_string(PrinterOptions::default())
        .unwrap_or_default();
    f64::from(parse_length_str(&css_text))
}
