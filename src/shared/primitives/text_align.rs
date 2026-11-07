/// Horizontal placement of a single-line text leaf inside its content box.
///
/// Only the three keywords the renderer can honour; `justify`, `start` and
/// `end` are deliberately unsupported (no wrapping, no RTL).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

impl TextAlign {
    /// How far to shift the glyphs right of the content box's left edge.
    ///
    /// Text wider than the box clamps to 0 so an overflowing line keeps its
    /// start visible instead of bleeding out to the left.
    #[must_use]
    pub fn offset(self, available_width: f32, text_width: f32) -> f32 {
        let free = (available_width - text_width).max(0.0);
        match self {
            Self::Left => 0.0,
            Self::Center => free / 2.0,
            Self::Right => free,
        }
    }
}

/// The width a text leaf may align within, paired with how to align in it.
/// Travels with `Canvas::draw_text` so the adapter, which alone knows the
/// shaped text width, can resolve the offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextAlignment {
    align: TextAlign,
    available_width: u32,
}

impl TextAlignment {
    #[must_use]
    pub const fn new(align: TextAlign, available_width: u32) -> Self {
        Self {
            align,
            available_width,
        }
    }

    #[must_use]
    pub const fn align(&self) -> TextAlign {
        self.align
    }

    #[must_use]
    pub const fn available_width(&self) -> u32 {
        self.available_width
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_left_never_shifts() {
        assert!(TextAlign::Left.offset(100.0, 40.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_center_splits_free_space() {
        assert!((TextAlign::Center.offset(100.0, 40.0) - 30.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_right_takes_all_free_space() {
        assert!((TextAlign::Right.offset(100.0, 40.0) - 60.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_overflowing_text_clamps_to_start() {
        for align in [TextAlign::Left, TextAlign::Center, TextAlign::Right] {
            assert!(align.offset(30.0, 80.0).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn test_default_is_left() {
        assert_eq!(TextAlign::default(), TextAlign::Left);
    }
}
