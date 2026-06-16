use resvg::usvg;
use std::path::Path;
use tiny_skia::{Transform};

pub fn load_icon_rgba(
    path: &Path,
    icon_size: u16,
    scale: f32,
) -> Option<(u32, u32, Vec<crate::domain::color::Color>)> {
    if path.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("svg")).unwrap_or(false) {
        let svg_data = std::fs::read(path).ok()?;
        let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default()).ok()?;
        let icon_px = ((icon_size as f32) * scale.max(1.0))
            .ceil()
            .max(icon_size as f32) as u32;
        let target = icon_px.max(1);
        let tree_size = tree.size();
        let sx = target as f32 / tree_size.width();
        let sy = target as f32 / tree_size.height();
        let fit_scale = sx.min(sy).max(0.001);
        let width = (tree_size.width() * fit_scale).ceil().max(1.0) as u32;
        let height = (tree_size.height() * fit_scale).ceil().max(1.0) as u32;
        let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
        let transform = Transform::from_scale(fit_scale, fit_scale);
        let mut pixmap_mut = pixmap.as_mut();
        resvg::render(&tree, transform, &mut pixmap_mut);

        let mut colors = Vec::with_capacity((width * height) as usize);
        for pixel in pixmap.data().chunks_exact(4) {
            let a = pixel[3];
            let (r, g, b) = if a == 0 {
                (0, 0, 0)
            } else {
                let unpremul =
                    |c: u8| -> u8 { (((c as u16 * 255) + (a as u16 / 2)) / a as u16).min(255) as u8 };
                (unpremul(pixel[0]), unpremul(pixel[1]), unpremul(pixel[2]))
            };
            colors.push(crate::domain::color::Color::new(r, g, b, a));
        }

        Some((width, height, colors))
    } else {
        let img = image::open(path).ok()?;
        let icon_px = ((icon_size as f32) * scale.max(1.0))
            .ceil()
            .max(icon_size as f32) as u32;
        let target = icon_px.max(1);
        let resized = image::imageops::resize(&img, target, target, image::imageops::FilterType::Lanczos3);
        
        let mut colors = Vec::with_capacity((resized.width() * resized.height()) as usize);
        for pixel in resized.pixels() {
            let [r, g, b, a] = pixel.0;
            colors.push(crate::domain::color::Color::new(r, g, b, a));
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
            "cranky-utils-test-{}-{}.svg",
            std::process::id(),
            nanos
        ))
    }

    fn temp_png_path() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "cranky-utils-test-{}-{}.png",
            std::process::id(),
            nanos
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
