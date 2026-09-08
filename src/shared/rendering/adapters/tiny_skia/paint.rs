use crate::shared::primitives::color::{Color as DomainColor, DrawingColor};
use cosmic_text::Family;
use tiny_skia::{
    Color as SkiaColor, GradientStop, LinearGradient, Paint, Point, Rect, SpreadMode, Transform,
};

pub(crate) fn to_skia_color(color: DomainColor) -> SkiaColor {
    SkiaColor::from_rgba8(color.b(), color.g(), color.r(), color.a())
}

pub(crate) fn get_paint(color: &DrawingColor, rect: Rect) -> Paint<'static> {
    let mut paint = Paint {
        anti_alias: true,
        ..Paint::default()
    };

    match color {
        DrawingColor::Solid(c) => {
            paint.set_color(to_skia_color(*c));
        }
        DrawingColor::Gradient(colors, angle) => {
            let count = colors.len().saturating_sub(1).max(1);
            let stops: Vec<GradientStop> = colors
                .iter()
                .enumerate()
                .map(|(i, &c)| {
                    let i_f = f32::from(u16::try_from(i).unwrap_or(u16::MAX));
                    let count_f = f32::from(u16::try_from(count).unwrap_or(1));
                    let pos = i_f / count_f;
                    GradientStop::new(pos, to_skia_color(c))
                })
                .collect();

            let angle_rad = angle.to_radians();
            let center_x = rect.left() + rect.width() / 2.0;
            let center_y = rect.top() + rect.height() / 2.0;

            let distance = (rect.width() / 2.0 * angle_rad.cos()).abs()
                + (rect.height() / 2.0 * angle_rad.sin()).abs();

            let x_offset = angle_rad.cos() * distance;
            let y_offset = angle_rad.sin() * distance;

            let start = Point::from_xy(center_x - x_offset, center_y - y_offset);
            let end = Point::from_xy(center_x + x_offset, center_y + y_offset);

            if let Some(shader) =
                LinearGradient::new(start, end, stops, SpreadMode::Pad, Transform::identity())
            {
                paint.shader = shader;
            } else if let Some(&c) = colors.first() {
                paint.set_color(to_skia_color(c));
            }
        }
    }
    paint
}

#[must_use]
pub const fn get_family(name: &str) -> Family<'_> {
    if name.eq_ignore_ascii_case("monospace") {
        Family::Monospace
    } else if name.eq_ignore_ascii_case("serif") {
        Family::Serif
    } else if name.eq_ignore_ascii_case("sans-serif") {
        Family::SansSerif
    } else if name.eq_ignore_ascii_case("cursive") {
        Family::Cursive
    } else if name.eq_ignore_ascii_case("fantasy") {
        Family::Fantasy
    } else if name.is_empty() {
        Family::Monospace
    } else {
        Family::Name(name)
    }
}
