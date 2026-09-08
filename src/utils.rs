use resvg::usvg;
use std::path::Path;
use tiny_skia::Transform;

#[must_use]
pub fn f32_to_u32(val: f32) -> u32 {
    if !val.is_finite() || val <= 0.0 {
        return 0;
    }
    let bits = val.to_bits();
    let exp_byte = u8::try_from((bits >> 23) & 0xFF).unwrap_or(0);
    let exp = i32::from(exp_byte).saturating_sub(127);
    if exp < 0 {
        return 0;
    }
    let mantissa = (bits & 0x007F_FFFF) | 0x0080_0000;
    if exp <= 23 {
        let shift = u32::try_from(23_i32.saturating_sub(exp)).unwrap_or(0);
        mantissa >> shift
    } else if exp < 32 {
        let shift = u32::try_from(exp.saturating_sub(23)).unwrap_or(0);
        mantissa << shift
    } else {
        u32::MAX
    }
}

#[must_use]
pub fn f32_to_i32(val: f32) -> i32 {
    if !val.is_finite() {
        return 0;
    }
    let is_neg = val.is_sign_negative();
    let u = f32_to_u32(val.abs());
    let i = i32::try_from(u).unwrap_or(i32::MAX);
    if is_neg {
        i.saturating_neg()
    } else {
        i
    }
}

#[must_use]
pub fn f64_to_u64(val: f64) -> u64 {
    if !val.is_finite() || val <= 0.0 {
        return 0;
    }
    let bits = val.to_bits();
    let exp_bits = u16::try_from((bits >> 52) & 0x07FF).unwrap_or(0);
    let exp = i32::from(exp_bits).saturating_sub(1023);
    if exp < 0 {
        return 0;
    }
    let mantissa = (bits & 0x000F_FFFF_FFFF_FFFF) | 0x0010_0000_0000_0000;
    if exp <= 52 {
        let shift = u32::try_from(52_i32.saturating_sub(exp)).unwrap_or(0);
        mantissa >> shift
    } else if exp < 64 {
        let shift = u32::try_from(exp.saturating_sub(52)).unwrap_or(0);
        mantissa << shift
    } else {
        u64::MAX
    }
}

#[must_use]
pub fn f64_to_i64(val: f64) -> i64 {
    if !val.is_finite() {
        return 0;
    }
    let is_neg = val.is_sign_negative();
    let u = f64_to_u64(val.abs());
    let i = i64::try_from(u).unwrap_or(i64::MAX);
    if is_neg {
        i.saturating_neg()
    } else {
        i
    }
}

#[must_use]
pub fn i64_to_f64(val: i64) -> f64 {
    i32::try_from(val).map_or_else(
        |_| {
            let hi = i32::try_from(val >> 32).unwrap_or(0);
            let lo = u32::try_from(val & 0xFFFF_FFFF).unwrap_or(0);
            f64::from(hi) * 4_294_967_296.0_f64 + f64::from(lo)
        },
        f64::from,
    )
}

#[must_use]
pub fn f64_to_f32(val: f64) -> f32 {
    if !val.is_finite() {
        if val.is_nan() {
            return f32::NAN;
        }
        return if val.is_sign_negative() {
            f32::NEG_INFINITY
        } else {
            f32::INFINITY
        };
    }
    if val == 0.0 {
        return if val.is_sign_negative() { -0.0 } else { 0.0 };
    }
    let bits = val.to_bits();
    let sign_bit = u32::try_from((bits >> 63) & 1).unwrap_or(0);
    let exp_val = i32::try_from((bits >> 52) & 0x7FF).unwrap_or(0).saturating_sub(1023);
    let new_exp = exp_val.saturating_add(127);
    if new_exp >= 255 {
        return if sign_bit == 1 {
            f32::NEG_INFINITY
        } else {
            f32::INFINITY
        };
    }
    if new_exp <= 0 {
        return 0.0;
    }
    let mantissa_bits = (bits & 0x000F_FFFF_FFFF_FFFF) >> 29;
    let mantissa = u32::try_from(mantissa_bits).unwrap_or(0);
    let new_exp_u32 = u32::try_from(new_exp).unwrap_or(0);
    let f32_bits = (sign_bit << 31) | (new_exp_u32 << 23) | (mantissa & 0x007F_FFFF);
    f32::from_bits(f32_bits)
}

#[must_use]
pub fn load_icon_rgba(path: &Path, icon_size: u16, scale: f32) -> Option<(u32, u32, Vec<u8>)> {
    let icon_px = f32_to_u32(
        (f32::from(icon_size) * scale.max(1.0))
            .ceil()
            .max(f32::from(icon_size)),
    );
    let target = icon_px.max(1);

    if path
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("svg"))
    {
        let svg_data = std::fs::read(path).ok()?;
        let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default()).ok()?;
        let tree_size = tree.size();
        let target_f32 = f32::from(u16::try_from(target).unwrap_or(u16::MAX));
        let sx = target_f32 / tree_size.width();
        let sy = target_f32 / tree_size.height();
        let fit_scale = sx.min(sy).max(0.001);
        let render_w = tree_size.width() * fit_scale;
        let render_h = tree_size.height() * fit_scale;
        let dx = (target_f32 - render_w) / 2.0;
        let dy = (target_f32 - render_h) / 2.0;

        let mut pixmap = tiny_skia::Pixmap::new(target, target)?;
        let transform = Transform::from_scale(fit_scale, fit_scale).post_translate(dx, dy);
        let mut pixmap_mut = pixmap.as_mut();
        resvg::render(&tree, transform, &mut pixmap_mut);

        let target_usize = usize::try_from(target).unwrap_or_default();
        let cap = target_usize.saturating_mul(target_usize).saturating_mul(4);
        let mut colors = Vec::with_capacity(cap);
        for chunk in pixmap.data().chunks_exact(4) {
            if let &[pr, pg, pb, a] = chunk {
                let (r, g, b) = if a == 0 {
                    (0, 0, 0)
                } else {
                    let unpremul = |c: u8| -> u8 {
                        let c_u16 = u16::from(c);
                        let a_u16 = u16::from(a);
                        let val = c_u16
                            .saturating_mul(255)
                            .saturating_add(a_u16 / 2)
                            .checked_div(a_u16)
                            .unwrap_or(0);
                        u8::try_from(val.min(255)).unwrap_or(255)
                    };
                    (unpremul(pr), unpremul(pg), unpremul(pb))
                };
                colors.push(r);
                colors.push(g);
                colors.push(b);
                colors.push(a);
            }
        }

        Some((target, target, colors))
    } else {
        let img = image::open(path).ok()?;
        let resized =
            image::imageops::resize(&img, target, target, image::imageops::FilterType::Lanczos3);

        let width_usize = usize::try_from(resized.width()).unwrap_or_default();
        let height_usize = usize::try_from(resized.height()).unwrap_or_default();
        let cap = width_usize.saturating_mul(height_usize).saturating_mul(4);
        let mut colors = Vec::with_capacity(cap);
        for pixel in resized.pixels() {
            let [r, g, b, a] = pixel.0;
            colors.push(r);
            colors.push(g);
            colors.push(b);
            colors.push(a);
        }

        Some((resized.width(), resized.height(), colors))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_svg_path() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "cranky-utils-test-{}-{nanos}.svg",
            std::process::id()
        ))
    }

    #[test]
    fn test_rasterize_svg_icon_rgba_missing_file() {
        let missing = std::env::temp_dir().join("definitely-missing-cranky.svg");
        assert!(load_icon_rgba(&missing, 16, 1.0).is_none());
    }

    #[test]
    fn test_rasterize_svg_icon_rgba_success() {
        let path = temp_svg_path();
        let svg = r#"<svg width="10" height="10"><rect width="10" height="10" fill="red"/></svg>"#;
        fs::write(&path, svg).unwrap();

        let rasterized = load_icon_rgba(&path, 16, 1.0);
        assert!(rasterized.is_some());
        let (w, h, data) = rasterized.unwrap();
        assert_eq!(w, 16);
        assert_eq!(h, 16);
        assert!(!data.is_empty());
    }

    #[test]
    fn test_rasterize_svg_icon_rgba_invalid_svg() {
        let path = temp_svg_path();
        fs::write(&path, "<svg><invalid></svg>").unwrap();

        let rasterized = load_icon_rgba(&path, 16, 1.0);
        assert!(rasterized.is_none());
    }

    #[test]
    fn test_load_icon_rgba_png_success() {
        let path = temp_svg_path().with_extension("png");
        let img = image::RgbaImage::new(8, 8);
        img.save(&path).unwrap();

        let loaded = load_icon_rgba(&path, 16, 2.0);
        assert!(loaded.is_some());
        let (w, h, data) = loaded.unwrap();
        assert_eq!(w, 32);
        assert_eq!(h, 32);
        assert!(!data.is_empty());
    }

    #[test]
    fn test_f32_to_u32_cases() {
        assert_eq!(f32_to_u32(-1.0), 0);
        assert_eq!(f32_to_u32(0.0), 0);
        assert_eq!(f32_to_u32(0.5), 0);
        assert_eq!(f32_to_u32(1.0), 1);
        assert_eq!(f32_to_u32(1.9), 1);
        assert_eq!(f32_to_u32(42.0), 42);
        assert_eq!(f32_to_u32(65535.0), 65535);
        assert_eq!(f32_to_u32(f32::NAN), 0);
        assert_eq!(f32_to_u32(f32::INFINITY), 0);
    }

    #[test]
    fn test_f32_to_i32_cases() {
        assert_eq!(f32_to_i32(0.0), 0);
        assert_eq!(f32_to_i32(1.5), 1);
        assert_eq!(f32_to_i32(-1.5), -1);
        assert_eq!(f32_to_i32(42.0), 42);
        assert_eq!(f32_to_i32(-42.0), -42);
        assert_eq!(f32_to_i32(f32::NAN), 0);
    }

    #[test]
    fn test_f64_to_f32_cases() {
        assert_eq!(f64_to_f32(0.0).to_bits(), 0.0f32.to_bits());
        assert_eq!(f64_to_f32(1.0).to_bits(), 1.0f32.to_bits());
        assert_eq!(f64_to_f32(42.5).to_bits(), 42.5f32.to_bits());
        assert!(f64_to_f32(f64::NAN).is_nan());
        assert_eq!(f64_to_f32(f64::INFINITY).to_bits(), f32::INFINITY.to_bits());
        assert_eq!(f64_to_f32(f64::NEG_INFINITY).to_bits(), f32::NEG_INFINITY.to_bits());
    }
}
