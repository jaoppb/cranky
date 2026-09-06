use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::color::DrawingColor;
use crate::shared::primitives::geometry::{LogicalPx, Position, Scale, Size};
use crate::shared::rendering::ports::canvas::Canvas;
use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::PixmapMut;

pub struct TinySkiaCosmicCanvas<'a> {
    pub(crate) pixmap: Option<PixmapMut<'a>>,
    pub(crate) font_system: &'a mut FontSystem,
    pub(crate) swash_cache: &'a mut SwashCache,
    pub(crate) scale: Scale,
    pub(crate) default_font_family: FontFamily,
    pub(crate) default_font_size: FontSize,
}

impl<'a> TinySkiaCosmicCanvas<'a> {
    #[must_use]
    pub const fn new(
        pixmap: PixmapMut<'a>,
        font_system: &'a mut FontSystem,
        swash_cache: &'a mut SwashCache,
        scale: Scale,
        default_font_family: FontFamily,
        default_font_size: FontSize,
    ) -> Self {
        Self {
            pixmap: Some(pixmap),
            font_system,
            swash_cache,
            scale,
            default_font_family,
            default_font_size,
        }
    }

    #[must_use]
    pub const fn from_optional_pixmap(
        pixmap: Option<PixmapMut<'a>>,
        font_system: &'a mut FontSystem,
        swash_cache: &'a mut SwashCache,
        scale: Scale,
        default_font_family: FontFamily,
        default_font_size: FontSize,
    ) -> Self {
        Self {
            pixmap,
            font_system,
            swash_cache,
            scale,
            default_font_family,
            default_font_size,
        }
    }
}

impl Canvas for TinySkiaCosmicCanvas<'_> {
    fn draw_rect(
        &mut self,
        x: LogicalPx,
        y: LogicalPx,
        width: LogicalPx,
        height: LogicalPx,
        color: DrawingColor,
        radius: LogicalPx,
    ) {
        self.draw_rect_impl(x, y, width, height, &color, radius);
    }

    fn draw_border(
        &mut self,
        position: Position,
        size: Size,
        color: DrawingColor,
        radius: LogicalPx,
        border_size: LogicalPx,
    ) {
        self.draw_border_impl(position, size, &color, radius, border_size);
    }

    fn draw_text(
        &mut self,
        text: &str,
        font_family: Option<&FontFamily>,
        font_size: Option<FontSize>,
        color: DrawingColor,
        position: Position,
    ) {
        self.draw_text_impl(text, font_family, font_size, &color, position);
    }

    fn draw_image(
        &mut self,
        image_data: &[u8],
        pixel_size: Size,
        logical_size: Size,
        position: Position,
    ) {
        self.draw_image_impl(image_data, pixel_size, logical_size, position);
    }
}
