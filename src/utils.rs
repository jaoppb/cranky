//! Saturating numeric conversions and icon loading into RGBA (SVG rasterised, other formats
//! decoded and resized).

use resvg::usvg;
use std::path::Path;
use tiny_skia::Transform;

#[must_use]
pub(crate) fn f32_to_u32(val: f32) -> u32 {
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
pub(crate) fn f32_to_i32(val: f32) -> i32 {
    if !val.is_finite() {
        return 0;
    }
    let is_neg = val.is_sign_negative();
    let magnitude = f32_to_u32(val.abs());
    let signed = i32::try_from(magnitude).unwrap_or(i32::MAX);
    if is_neg {
        signed.saturating_neg()
    } else {
        signed
    }
}

#[must_use]
pub(crate) fn f64_to_u64(val: f64) -> u64 {
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
pub(crate) fn f64_to_i64(val: f64) -> i64 {
    if !val.is_finite() {
        return 0;
    }
    let is_neg = val.is_sign_negative();
    let magnitude = f64_to_u64(val.abs());
    let signed = i64::try_from(magnitude).unwrap_or(i64::MAX);
    if is_neg {
        signed.saturating_neg()
    } else {
        signed
    }
}

#[must_use]
pub(crate) fn i64_to_f64(val: i64) -> f64 {
    i32::try_from(val).map_or_else(
        |_| {
            let high = i32::try_from(val >> 32).unwrap_or(0);
            let low = u32::try_from(val & 0xFFFF_FFFF).unwrap_or(0);
            f64::mul_add(f64::from(high), 4_294_967_296.0_f64, f64::from(low))
        },
        f64::from,
    )
}

#[must_use]
pub(crate) fn f64_to_f32(val: f64) -> f32 {
    if val.is_nan() {
        f32::NAN
    } else if val.is_infinite() {
        infinity_with_sign_of(val)
    } else if val == 0.0 {
        if val.is_sign_negative() { -0.0 } else { 0.0 }
    } else {
        narrow_finite_nonzero(val)
    }
}

/// The `f32` infinity with the sign of `val`.
const fn infinity_with_sign_of(val: f64) -> f32 {
    if val.is_sign_negative() {
        f32::NEG_INFINITY
    } else {
        f32::INFINITY
    }
}

/// Re-encodes a finite, non-zero `f64` as an `f32`: infinity past the `f32` range, zero below
/// its normal range, the mantissa truncated otherwise.
fn narrow_finite_nonzero(val: f64) -> f32 {
    let bits = val.to_bits();
    let sign_bit = u32::try_from((bits >> 63) & 1).unwrap_or(0);
    let exp_val = i32::try_from((bits >> 52) & 0x7FF)
        .unwrap_or(0)
        .saturating_sub(1023);
    let new_exp = exp_val.saturating_add(127);
    if new_exp >= 255 {
        return infinity_with_sign_of(val);
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
pub(crate) fn load_icon_rgba(
    path: &Path,
    icon_size: u16,
    scale: f32,
) -> Option<(u32, u32, Vec<u8>)> {
    let target = icon_side_px(icon_size, scale);
    if is_svg(path) {
        rasterize_svg(path, target)
    } else {
        decode_bitmap(path, target)
    }
}

/// The side of the square icon in pixels: `icon_size` scaled by `scale` (never below 1),
/// rounded up, and at least one pixel.
fn icon_side_px(icon_size: u16, scale: f32) -> u32 {
    let icon_px = f32_to_u32(
        (f32::from(icon_size) * scale.max(1.0))
            .ceil()
            .max(f32::from(icon_size)),
    );
    icon_px.max(1)
}

/// Whether `path` names an SVG file, whatever the case of its extension.
fn is_svg(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"))
}

/// Rasterises the SVG at `path` into a `target`-pixel square as straight RGBA.
fn rasterize_svg(path: &Path, target: u32) -> Option<(u32, u32, Vec<u8>)> {
    let svg_data = std::fs::read(path).ok()?;
    let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default()).ok()?;
    let pixmap = render_fitted(&tree, target)?;
    Some((target, target, unpremultiplied_rgba(pixmap.data())))
}

/// Renders `tree` scaled to fit a `target`-pixel square, centred in it.
fn render_fitted(tree: &usvg::Tree, target: u32) -> Option<tiny_skia::Pixmap> {
    let tree_size = tree.size();
    let target_f32 = f32::from(u16::try_from(target).unwrap_or(u16::MAX));
    let horizontal_scale = target_f32 / tree_size.width();
    let vertical_scale = target_f32 / tree_size.height();
    let fit_scale = horizontal_scale.min(vertical_scale).max(0.001);
    let render_w = tree_size.width() * fit_scale;
    let render_h = tree_size.height() * fit_scale;
    let dx = (target_f32 - render_w) / 2.0;
    let dy = (target_f32 - render_h) / 2.0;

    let mut pixmap = tiny_skia::Pixmap::new(target, target)?;
    let transform = Transform::from_scale(fit_scale, fit_scale).post_translate(dx, dy);
    let mut pixmap_mut = pixmap.as_mut();
    resvg::render(tree, transform, &mut pixmap_mut);
    Some(pixmap)
}

/// The premultiplied RGBA `data` as straight RGBA.
fn unpremultiplied_rgba(data: &[u8]) -> Vec<u8> {
    let mut colors = Vec::with_capacity(data.len());
    for &[
        premultiplied_red,
        premultiplied_green,
        premultiplied_blue,
        alpha,
    ] in data.as_chunks::<4>().0
    {
        let (red, green, blue) = if alpha == 0 {
            (0, 0, 0)
        } else {
            (
                unpremultiply(premultiplied_red, alpha),
                unpremultiply(premultiplied_green, alpha),
                unpremultiply(premultiplied_blue, alpha),
            )
        };
        colors.push(red);
        colors.push(green);
        colors.push(blue);
        colors.push(alpha);
    }
    colors
}

/// The straight value of `channel`, premultiplied by a non-zero `alpha`, rounded to nearest.
fn unpremultiply(channel: u8, alpha: u8) -> u8 {
    let c_u16 = u16::from(channel);
    let a_u16 = u16::from(alpha);
    let val = c_u16
        .saturating_mul(255)
        .saturating_add(a_u16 / 2)
        .checked_div(a_u16)
        .unwrap_or(0);
    u8::try_from(val.min(255)).unwrap_or(255)
}

/// Decodes the bitmap at `path` and resizes it to a `target`-pixel square as RGBA.
fn decode_bitmap(path: &Path, target: u32) -> Option<(u32, u32, Vec<u8>)> {
    let img = image::open(path).ok()?;
    let resized =
        image::imageops::resize(&img, target, target, image::imageops::FilterType::Lanczos3);

    let width_usize = usize::try_from(resized.width()).unwrap_or_default();
    let height_usize = usize::try_from(resized.height()).unwrap_or_default();
    let cap = width_usize.saturating_mul(height_usize).saturating_mul(4);
    let mut colors = Vec::with_capacity(cap);
    for pixel in resized.pixels() {
        let [red, green, blue, alpha] = pixel.0;
        colors.push(red);
        colors.push(green);
        colors.push(blue);
        colors.push(alpha);
    }

    Some((resized.width(), resized.height(), colors))
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
        let (width, height, data) = rasterized.unwrap();
        assert_eq!(width, 16);
        assert_eq!(height, 16);
        assert_eq!(data.len(), 16 * 16 * 4);
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
        let (width, height, data) = loaded.unwrap();
        assert_eq!(width, 32);
        assert_eq!(height, 32);
        assert_eq!(data.len(), 32 * 32 * 4);
    }

    /// Writes `svg` to a fresh temporary `.svg` file and loads it as a 16px icon at scale 1.
    fn load_svg_icon(svg: &str) -> (u32, u32, Vec<u8>) {
        let path = temp_svg_path();
        fs::write(&path, svg).unwrap();
        let loaded = load_icon_rgba(&path, 16, 1.0).unwrap();
        let _ = fs::remove_file(&path);
        loaded
    }

    /// The RGBA bytes of the pixel at `(x, y)` in a `width`-pixel-wide RGBA buffer.
    fn pixel_at(data: &[u8], width: u32, (x, y): (u32, u32)) -> [u8; 4] {
        let row_bytes = usize::try_from(width).unwrap().checked_mul(4).unwrap();
        let row = data
            .chunks_exact(row_bytes)
            .nth(usize::try_from(y).unwrap())
            .unwrap();
        row.as_chunks::<4>().0[usize::try_from(x).unwrap()]
    }

    #[test]
    fn test_svg_icon_is_straight_rgba_in_channel_order() {
        let (width, _, data) = load_svg_icon(
            r##"<svg width="10" height="10"><rect width="10" height="10" fill="#ff8000"/></svg>"##,
        );
        assert_eq!(pixel_at(&data, width, (0, 0)), [255, 128, 0, 255]);
        assert_eq!(pixel_at(&data, width, (15, 15)), [255, 128, 0, 255]);
    }

    /// A half-transparent fill comes back un-premultiplied: the colour at full strength and the
    /// alpha at half.
    #[test]
    fn test_svg_icon_is_unpremultiplied() {
        let (width, _, data) = load_svg_icon(
            r#"<svg width="10" height="10"><rect width="10" height="10" fill="red" fill-opacity="0.5"/></svg>"#,
        );
        assert_eq!(pixel_at(&data, width, (8, 8)), [255, 0, 0, 128]);
    }

    /// Un-premultiplying rounds to nearest: mid grey at half opacity is stored as 64 over 128,
    /// which comes back as 128, where truncating would give 127.
    #[test]
    fn test_svg_icon_unpremultiplies_to_the_nearest_value() {
        let (width, _, data) = load_svg_icon(
            r##"<svg width="10" height="10"><rect width="10" height="10" fill="#808080" fill-opacity="0.5"/></svg>"##,
        );
        assert_eq!(pixel_at(&data, width, (8, 8)), [128, 128, 128, 128]);
    }

    /// A wide SVG is scaled to fit the square and centred vertically, leaving transparent
    /// bands above and below.
    #[test]
    fn test_svg_icon_is_fitted_and_centred() {
        let (width, height, data) = load_svg_icon(
            r#"<svg width="20" height="10"><rect width="20" height="10" fill="red"/></svg>"#,
        );
        assert_eq!((width, height), (16, 16));
        assert_eq!(pixel_at(&data, width, (8, 0)), [0, 0, 0, 0]);
        assert_eq!(pixel_at(&data, width, (8, 3)), [0, 0, 0, 0]);
        assert_eq!(pixel_at(&data, width, (8, 4)), [255, 0, 0, 255]);
        assert_eq!(pixel_at(&data, width, (8, 11)), [255, 0, 0, 255]);
        assert_eq!(pixel_at(&data, width, (8, 12)), [0, 0, 0, 0]);
        assert_eq!(pixel_at(&data, width, (8, 15)), [0, 0, 0, 0]);
    }

    /// The icon is never smaller than its nominal size, and a fractional scale rounds up.
    #[test]
    fn test_icon_size_scales_up_and_rounds_up() {
        let path = temp_svg_path();
        fs::write(
            &path,
            r#"<svg width="10" height="10"><rect width="10" height="10" fill="red"/></svg>"#,
        )
        .unwrap();
        let side = |scale: f32| load_icon_rgba(&path, 16, scale).unwrap().0;
        assert_eq!(side(0.5), 16);
        assert_eq!(side(1.5), 24);
        assert_eq!(side(1.01), 17);
        let _ = fs::remove_file(&path);
    }

    /// The extension test ignores case: an `.SVG` file is rasterised, not decoded as a bitmap.
    #[test]
    fn test_svg_extension_is_case_insensitive() {
        let path = temp_svg_path().with_extension("SVG");
        fs::write(
            &path,
            r#"<svg width="10" height="10"><rect width="10" height="10" fill="red"/></svg>"#,
        )
        .unwrap();
        let loaded = load_icon_rgba(&path, 16, 1.0);
        let _ = fs::remove_file(&path);
        let (width, height, data) = loaded.unwrap();
        assert_eq!((width, height), (16, 16));
        assert_eq!(pixel_at(&data, width, (0, 0)), [255, 0, 0, 255]);
    }

    #[test]
    fn test_bitmap_icon_keeps_its_channel_order() {
        let path = temp_svg_path().with_extension("png");
        image::RgbaImage::from_pixel(8, 8, image::Rgba([10, 20, 30, 255]))
            .save(&path)
            .unwrap();
        let loaded = load_icon_rgba(&path, 4, 1.0);
        let _ = fs::remove_file(&path);
        let (width, height, data) = loaded.unwrap();
        assert_eq!((width, height), (4, 4));
        assert_eq!(pixel_at(&data, width, (0, 0)), [10, 20, 30, 255]);
        assert_eq!(pixel_at(&data, width, (3, 3)), [10, 20, 30, 255]);
    }

    #[test]
    fn test_bitmap_icon_missing_file() {
        let missing = std::env::temp_dir().join("definitely-missing-cranky.png");
        assert!(load_icon_rgba(&missing, 16, 1.0).is_none());
    }

    #[test]
    fn test_f64_to_f32_out_of_range() {
        for (input, expected) in [
            (1e300, f32::INFINITY),
            (-1e300, f32::NEG_INFINITY),
            (5.0e38, f32::INFINITY),
            (1e-300, 0.0),
            (-0.0, -0.0),
            (-42.5, -42.5),
            (f64::from(f32::MAX), f32::MAX),
            (f64::from(f32::MIN_POSITIVE), f32::MIN_POSITIVE),
        ] {
            assert_eq!(f64_to_f32(input).to_bits(), expected.to_bits(), "{input}");
        }
    }

    // Known bug: a negative value too small for an `f32` loses its sign (`-0.0` expected), and
    // the subnormal `f32` range is flushed to zero.
    #[test]
    fn test_f64_to_f32_underflow_is_positive_zero() {
        assert_eq!(f64_to_f32(-1e-300).to_bits(), 0.0f32.to_bits());
        let subnormal = f64::from(f32::MIN_POSITIVE) * 0.75;
        assert_eq!(f64_to_f32(subnormal).to_bits(), 0.0f32.to_bits());
    }

    // Known bug: the mantissa is truncated, not rounded to nearest, so 0.1 lands one ulp below
    // `0.1_f32`.
    #[test]
    fn test_f64_to_f32_truncates_the_mantissa() {
        assert_eq!(0.1f32.to_bits(), 0x3DCC_CCCD);
        assert_eq!(f64_to_f32(0.1).to_bits(), 0x3DCC_CCCC);
    }

    #[test]
    fn test_f32_to_u32_cases() {
        for (input, expected) in [
            (-1.0, 0),
            (0.0, 0),
            (0.5, 0),
            (1.0, 1),
            (1.9, 1),
            (42.0, 42),
            (65535.0, 65535),
            (f32::NAN, 0),
            (f32::INFINITY, 0),
        ] {
            assert_eq!(f32_to_u32(input), expected, "{input}");
        }
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
        assert_eq!(
            f64_to_f32(f64::NEG_INFINITY).to_bits(),
            f32::NEG_INFINITY.to_bits()
        );
    }
}
