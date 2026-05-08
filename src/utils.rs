use resvg::usvg;
use std::path::Path;
use tiny_skia::{Transform};

pub fn rasterize_svg_icon_rgba(
    path: &Path,
    icon_size: u16,
    scale: f32,
) -> Option<image::RgbaImage> {
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

    let mut rgba = image::RgbaImage::new(width, height);
    for (idx, pixel) in pixmap.data().chunks_exact(4).enumerate() {
        let a = pixel[3];
        let (r, g, b) = if a == 0 {
            (0, 0, 0)
        } else {
            let unpremul =
                |c: u8| -> u8 { (((c as u16 * 255) + (a as u16 / 2)) / a as u16).min(255) as u8 };
            (unpremul(pixel[0]), unpremul(pixel[1]), unpremul(pixel[2]))
        };
        let x = (idx as u32) % width;
        let y = (idx as u32) / width;
        rgba.put_pixel(x, y, image::Rgba([r, g, b, a]));
    }

    Some(rgba)
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

    #[test]
    fn test_rasterize_svg_icon_rgba_missing_file() {
        let missing = std::env::temp_dir().join("definitely-missing-cranky.svg");
        assert!(rasterize_svg_icon_rgba(&missing, 16, 1.0).is_none());
    }

    #[test]
    fn test_rasterize_svg_icon_rgba_success() {
        let path = temp_svg_path();
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><circle cx="12" cy="12" r="10" fill="#ff00ff"/></svg>"##;
        fs::write(&path, svg).unwrap();

        let rasterized = rasterize_svg_icon_rgba(&path, 16, 1.0);
        let _ = fs::remove_file(&path);

        assert!(rasterized.is_some());
        let image = rasterized.unwrap();
        assert!(image.width() > 0);
        assert!(image.height() > 0);
    }

    #[test]
    fn test_rasterize_svg_icon_rgba_invalid_svg() {
        let path = temp_svg_path();
        fs::write(&path, "this is not svg").unwrap();

        let rasterized = rasterize_svg_icon_rgba(&path, 16, 1.0);
        let _ = fs::remove_file(&path);

        assert!(rasterized.is_none());
    }
}
