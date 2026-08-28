use resvg::usvg;
use std::path::Path;
use tiny_skia::Transform;

#[must_use]
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
pub fn load_icon_rgba(path: &Path, icon_size: u16, scale: f32) -> Option<(u32, u32, Vec<u8>)> {
    let icon_px = (f32::from(icon_size) * scale.max(1.0))
        .ceil()
        .max(f32::from(icon_size)) as u32;
    let target = icon_px.max(1);

    if path
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("svg"))
    {
        let svg_data = std::fs::read(path).ok()?;
        let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default()).ok()?;
        let tree_size = tree.size();
        let target_f32 = target as f32;
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
}
