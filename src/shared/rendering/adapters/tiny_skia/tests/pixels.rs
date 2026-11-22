//! Pins every pixel the canvas paints for rectangles, borders, gradients and images, so a change
//! to the drawing code that moves a single pixel fails here. A digest only changes with an
//! intended change to how shapes are painted.
use super::target;
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::color::{Color, DrawingColor};
use crate::shared::primitives::geometry::{LogicalPx, Position, Rect, Size};
use crate::shared::rendering::adapters::tiny_skia::canvas::TinySkiaCosmicCanvas;
use crate::shared::rendering::adapters::tiny_skia::factory::FontCache;
use crate::shared::rendering::ports::canvas::Canvas;
use crate::shared::rendering::ports::draw::{BorderStroke, RgbaImage, RoundedRect};
use crate::shared::rendering::ports::font::Font;
use tiny_skia::Pixmap;

/// A 64x48 pixmap after `draw` ran on a canvas over it at scale 1.5.
fn painted(draw: impl FnOnce(&mut TinySkiaCosmicCanvas<'_>)) -> Pixmap {
    let mut pixmap = Pixmap::new(64, 48).unwrap();
    let mut fonts = FontCache::new();
    {
        let font = Font::new(
            FontFamily::new("sans-serif".to_string()),
            FontSize::new(14.0),
        );
        let mut canvas = TinySkiaCosmicCanvas::new(target(&mut pixmap, 1.5), &mut fonts, font);
        draw(&mut canvas);
    }
    pixmap
}

/// FNV-1a over every byte of `pixmap`.
fn digest(pixmap: &Pixmap) -> u64 {
    pixmap
        .data()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325, |hash, &byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        })
}

fn shape(origin: (i32, i32), size: (u32, u32), radius: f32) -> RoundedRect {
    let area = Rect::new(Position::new(origin.0, origin.1), Size::new(size.0, size.1));
    RoundedRect::new(area, LogicalPx::new(radius))
}

fn red() -> DrawingColor {
    DrawingColor::Solid(Color::rgb(255, 0, 0))
}

fn red_to_green(angle: f32) -> DrawingColor {
    DrawingColor::Gradient(vec![Color::rgb(255, 0, 0), Color::rgb(0, 255, 0)], angle)
}

#[test]
fn a_square_rect_paints_the_same_pixels() {
    let pixmap = painted(|canvas| canvas.draw_rect(shape((3, 2), (30, 20), 0.0), red()));
    assert_eq!(digest(&pixmap), 0xb574_c10e_d0d7_e815);
}

#[test]
fn a_rounded_gradient_rect_paints_the_same_pixels() {
    let pixmap =
        painted(|canvas| canvas.draw_rect(shape((3, 2), (30, 20), 6.0), red_to_green(45.0)));
    assert_eq!(digest(&pixmap), 0x040b_1290_bffd_51a0);
}

#[test]
fn a_rect_radius_past_half_its_size_is_clamped() {
    let pixmap = painted(|canvas| canvas.draw_rect(shape((3, 2), (30, 20), 40.0), red()));
    assert_eq!(digest(&pixmap), 0xcf7f_331f_71c1_3c71);
}

#[test]
fn a_tall_rect_radius_past_half_its_width_is_clamped() {
    let pixmap = painted(|canvas| canvas.draw_rect(shape((3, 2), (10, 30), 40.0), red()));
    assert_eq!(digest(&pixmap), 0x8f7e_4fa7_b531_84cf);
}

#[test]
fn a_three_stop_gradient_paints_the_same_pixels() {
    let colors = vec![
        Color::rgb(255, 0, 0),
        Color::rgb(0, 255, 0),
        Color::rgb(0, 0, 255),
    ];
    let color = DrawingColor::Gradient(colors, 90.0);
    let pixmap = painted(|canvas| canvas.draw_rect(shape((0, 0), (40, 30), 0.0), color));
    assert_eq!(digest(&pixmap), 0x96ae_9b6c_2434_3805);
}

#[test]
fn a_gradient_with_no_direction_paints_its_first_color() {
    let gradient =
        painted(|canvas| canvas.draw_rect(shape((3, 2), (30, 20), 6.0), red_to_green(f32::NAN)));
    let solid = painted(|canvas| canvas.draw_rect(shape((3, 2), (30, 20), 6.0), red()));
    assert_eq!(gradient.data(), solid.data());
}

#[test]
fn a_square_border_paints_the_same_pixels() {
    let stroke = BorderStroke::new(red(), LogicalPx::new(3.0));
    let pixmap = painted(|canvas| canvas.draw_border(shape((3, 2), (30, 20), 0.0), stroke));
    assert_eq!(digest(&pixmap), 0x5c4f_7f9a_d7ba_f1dd);
}

#[test]
fn a_rounded_gradient_border_paints_the_same_pixels() {
    let stroke = BorderStroke::new(red_to_green(90.0), LogicalPx::new(2.0));
    let pixmap = painted(|canvas| canvas.draw_border(shape((3, 2), (30, 20), 8.0), stroke));
    assert_eq!(digest(&pixmap), 0x2142_cd8e_bd8b_7613);
}

#[test]
fn a_tall_border_radius_past_half_its_width_is_clamped() {
    let stroke = BorderStroke::new(red(), LogicalPx::new(2.0));
    let pixmap = painted(|canvas| canvas.draw_border(shape((3, 2), (10, 30), 40.0), stroke));
    assert_eq!(digest(&pixmap), 0x3d8b_3a9f_496f_08f7);
}

#[test]
fn a_border_wider_than_its_box_paints_the_same_pixels() {
    let stroke = BorderStroke::new(red(), LogicalPx::new(12.0));
    let pixmap = painted(|canvas| canvas.draw_border(shape((3, 2), (10, 30), 4.0), stroke));
    assert_eq!(digest(&pixmap), 0xbaf4_0b0d_54b7_dbf7);
}

#[test]
fn a_wide_border_radius_past_half_its_height_is_clamped() {
    let stroke = BorderStroke::new(red(), LogicalPx::new(2.0));
    let pixmap = painted(|canvas| canvas.draw_border(shape((3, 2), (40, 10), 9.0), stroke));
    assert_eq!(digest(&pixmap), 0xd288_c793_d857_627b);
}

#[test]
fn a_border_taller_than_its_box_paints_the_same_pixels() {
    let stroke = BorderStroke::new(red(), LogicalPx::new(12.0));
    let pixmap = painted(|canvas| canvas.draw_border(shape((3, 2), (30, 10), 4.0), stroke));
    assert_eq!(digest(&pixmap), 0x5c7f_b29c_fc75_c49d);
}

#[test]
fn a_border_radius_under_half_its_stroke_draws_square_corners() {
    let rounded = BorderStroke::new(red(), LogicalPx::new(4.0));
    let square = BorderStroke::new(red(), LogicalPx::new(4.0));
    let small_radius = painted(|canvas| canvas.draw_border(shape((3, 2), (30, 20), 1.5), rounded));
    let no_radius = painted(|canvas| canvas.draw_border(shape((3, 2), (30, 20), 0.0), square));
    assert_eq!(small_radius.data(), no_radius.data());
}

#[test]
fn a_border_without_width_paints_nothing() {
    let stroke = BorderStroke::new(red(), LogicalPx::new(0.0));
    let pixmap = painted(|canvas| canvas.draw_border(shape((3, 2), (30, 20), 4.0), stroke));
    assert!(pixmap.data().iter().all(|&byte| byte == 0));
}

#[test]
fn a_scaled_translucent_image_paints_the_same_pixels() {
    let rgba = [
        255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 64, 255, 255, 255, 0,
    ];
    let area = Rect::new(Position::new(4, 3), Size::new(20, 10));
    let image = RgbaImage::new(&rgba, Size::new(2, 2));
    let pixmap = painted(|canvas| canvas.draw_image(image, area));
    assert_eq!(digest(&pixmap), 0xa4d1_d510_565c_3731);
}

#[test]
fn an_image_shorter_than_its_size_paints_nothing() {
    let area = Rect::new(Position::new(0, 0), Size::new(20, 10));
    let image = RgbaImage::new(&[255, 0, 0, 255], Size::new(2, 2));
    let pixmap = painted(|canvas| canvas.draw_image(image, area));
    assert!(pixmap.data().iter().all(|&byte| byte == 0));
}
