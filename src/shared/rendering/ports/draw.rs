//! The values a canvas draws from: the buffer it paints into, shapes, strokes and images.
use crate::shared::primitives::color::DrawingColor;
use crate::shared::primitives::geometry::{LogicalPx, Rect, Scale, Size};

/// The pixel buffer a canvas paints into: RGBA bytes laid out in rows of `size` physical pixels,
/// and the scale from the logical pixels draw requests use to those physical pixels.
#[derive(Debug)]
pub struct PaintTarget<'a> {
    pixels: &'a mut [u8],
    size: Size,
    scale: Scale,
}

impl<'a> PaintTarget<'a> {
    /// Targets `pixels`, a buffer of `size` physical pixels drawn into at `scale`.
    #[must_use]
    pub const fn new(pixels: &'a mut [u8], size: Size, scale: Scale) -> Self {
        Self {
            pixels,
            size,
            scale,
        }
    }

    #[must_use]
    pub(crate) const fn size(&self) -> Size {
        self.size
    }

    #[must_use]
    pub(crate) const fn scale(&self) -> Scale {
        self.scale
    }

    #[must_use]
    pub(crate) const fn into_pixels(self) -> &'a mut [u8] {
        self.pixels
    }
}

/// A rectangle in logical pixels with rounded corners of one radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoundedRect {
    x: LogicalPx,
    y: LogicalPx,
    width: LogicalPx,
    height: LogicalPx,
    radius: LogicalPx,
}

impl RoundedRect {
    /// Covers `rect` with corners of `radius`; coordinates outside `i16` and sizes outside `u16`
    /// count as 0.
    #[must_use]
    pub fn new(rect: Rect, radius: LogicalPx) -> Self {
        Self {
            x: LogicalPx::new(f32::from(i16::try_from(rect.x()).unwrap_or(0))),
            y: LogicalPx::new(f32::from(i16::try_from(rect.y()).unwrap_or(0))),
            width: LogicalPx::new(f32::from(u16::try_from(rect.width()).unwrap_or(0))),
            height: LogicalPx::new(f32::from(u16::try_from(rect.height()).unwrap_or(0))),
            radius,
        }
    }

    /// The part of this rectangle that keeps its left edge and is `width` wide.
    #[must_use]
    pub(crate) const fn with_width(self, width: LogicalPx) -> Self {
        Self { width, ..self }
    }

    /// The part of this rectangle that keeps its bottom edge and is `height` tall.
    #[must_use]
    pub(crate) fn with_bottom_height(self, height: LogicalPx) -> Self {
        Self {
            y: LogicalPx::new(self.y.value() + (self.height.value() - height.value())),
            height,
            ..self
        }
    }

    #[must_use]
    pub(crate) const fn x(&self) -> LogicalPx {
        self.x
    }

    #[must_use]
    pub(crate) const fn y(&self) -> LogicalPx {
        self.y
    }

    #[must_use]
    pub(crate) const fn width(&self) -> LogicalPx {
        self.width
    }

    #[must_use]
    pub(crate) const fn height(&self) -> LogicalPx {
        self.height
    }

    #[must_use]
    pub(crate) const fn radius(&self) -> LogicalPx {
        self.radius
    }
}

/// How a border is stroked: its color and its width in logical pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct BorderStroke {
    color: DrawingColor,
    width: LogicalPx,
}

impl BorderStroke {
    /// A stroke of `width` in `color`.
    #[must_use]
    pub const fn new(color: DrawingColor, width: LogicalPx) -> Self {
        Self { color, width }
    }

    #[must_use]
    pub(crate) const fn color(&self) -> &DrawingColor {
        &self.color
    }

    #[must_use]
    pub(crate) const fn width(&self) -> LogicalPx {
        self.width
    }
}

/// A decoded image: straight RGBA bytes in rows of `size` pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbaImage<'a> {
    data: &'a [u8],
    size: Size,
}

impl<'a> RgbaImage<'a> {
    /// The image whose `size` pixels are `data`.
    #[must_use]
    pub(crate) const fn new(data: &'a [u8], size: Size) -> Self {
        Self { data, size }
    }

    #[must_use]
    pub(crate) const fn data(&self) -> &'a [u8] {
        self.data
    }

    #[must_use]
    pub(crate) const fn size(&self) -> Size {
        self.size
    }
}
