#[cfg(test)]
#[macro_export]
macro_rules! assert_pixel_color {
    ($pixmap:expr, $x:expr, $y:expr, $expected_color:expr) => {
        let (r, g, b, a) = $crate::test_utils::get_pixel_color(&mut $pixmap, $x, $y);
        let expected = $expected_color;
        let actual = tiny_skia::Color::from_rgba8(r, g, b, a);

        let diff_r = (actual.red() - expected.red()).abs();
        let diff_g = (actual.green() - expected.green()).abs();
        let diff_b = (actual.blue() - expected.blue()).abs();
        let diff_a = (actual.alpha() - expected.alpha()).abs();

        let tolerance = 0.01;
        if diff_r > tolerance || diff_g > tolerance || diff_b > tolerance || diff_a > tolerance {
            panic!(
                "Pixel at ({}, {}) color mismatch.\nActual: {actual:?}\nExpected: {expected:?}",
                $x, $y
            );
        }
    };
}

#[cfg(test)]
#[macro_export]
macro_rules! assert_pixmap_has_color {
    ($pixmap:expr, $expected_color:expr) => {
        let mut found = false;
        let expected = $expected_color;
        let data = $pixmap.data_mut();
        for chunk in data.chunks_exact(4) {
            if let &[r, g, b, a] = chunk {
                let actual = tiny_skia::Color::from_rgba8(r, g, b, a);
                let diff_r = (actual.red() - expected.red()).abs();
                let diff_g = (actual.green() - expected.green()).abs();
                let diff_b = (actual.blue() - expected.blue()).abs();
                let diff_a = (actual.alpha() - expected.alpha()).abs();
                let tolerance = 0.01;
                if diff_r <= tolerance
                    && diff_g <= tolerance
                    && diff_b <= tolerance
                    && diff_a <= tolerance
                {
                    found = true;
                    break;
                }
            }
        }
        if !found {
            panic!("Color {expected:?} not found in pixmap");
        }
    };
}

#[cfg(test)]
/// Extracts RGBA pixel color at `(x, y)` from a pixmap.
///
/// # Panics
///
/// Panics if pixel coordinates are out of bounds.
#[must_use]
pub fn get_pixel_color(pixmap: &mut tiny_skia::PixmapMut, x: u32, y: u32) -> (u8, u8, u8, u8) {
    let width = pixmap.width();
    let height = pixmap.height();
    assert!(x < width && y < height, "Pixel coordinates ({x}, {y}) out of bounds");
    let data = pixmap.data_mut();
    let y_idx = usize::try_from(y).unwrap_or_default();
    let width_idx = usize::try_from(width).unwrap_or_default();
    let x_idx = usize::try_from(x).unwrap_or_default();
    let idx = y_idx
        .saturating_mul(width_idx)
        .saturating_add(x_idx)
        .saturating_mul(4);
    (
        data[idx],
        data[idx.saturating_add(1)],
        data[idx.saturating_add(2)],
        data[idx.saturating_add(3)],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiny_skia::*;

    #[test]
    fn test_get_pixel_color() {
        let mut pixmap_data = vec![0; 400];
        let mut pixmap = PixmapMut::from_bytes(&mut pixmap_data, 10, 10).unwrap();
        pixmap.fill(Color::from_rgba8(255, 128, 64, 255));

        let (r, g, b, a) = get_pixel_color(&mut pixmap, 5, 5);
        assert_eq!(r, 255);
        assert_eq!(g, 128);
        assert_eq!(b, 64);
        assert_eq!(a, 255);
    }

    #[test]
    #[should_panic(expected = "out of bounds")]
    fn test_get_pixel_color_bounds() {
        let mut pixmap_data = vec![0; 400];
        let mut pixmap = PixmapMut::from_bytes(&mut pixmap_data, 10, 10).unwrap();
        let _ = get_pixel_color(&mut pixmap, 10, 10);
    }

    #[test]
    fn test_assert_macros() {
        let mut pixmap_data = vec![0; 400];
        let mut pixmap = PixmapMut::from_bytes(&mut pixmap_data, 10, 10).unwrap();
        let color = Color::from_rgba8(100, 200, 50, 255);
        pixmap.fill(color);

        assert_pixel_color!(pixmap, 0, 0, color);
        assert_pixmap_has_color!(pixmap, color);
    }
}
