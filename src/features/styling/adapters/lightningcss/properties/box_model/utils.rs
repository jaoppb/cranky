use crate::features::styling::domain::CssLength;
use lightningcss::stylesheet::PrinterOptions;
use lightningcss::traits::ToCss;
use lightningcss::values::length::LengthPercentageOrAuto;

#[must_use]
pub fn parse_size_str(s: &str) -> Option<CssLength> {
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

#[must_use]
pub fn parse_length_str(s: &str) -> f32 {
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

#[must_use]
pub fn length_to_f64(len: &LengthPercentageOrAuto) -> f64 {
    let s = len
        .to_css_string(PrinterOptions::default())
        .unwrap_or_default();
    f64::from(parse_length_str(&s))
}
