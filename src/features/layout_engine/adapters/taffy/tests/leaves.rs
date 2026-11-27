//! Text leaves: measured size, padding around the glyphs, and the leaf's own font.
use super::support::{flex_children, flex_node, lay_out, root_path, text_leaf};
use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::layout_engine::domain::{BoxMargin, LayoutRoot, TextMeasurer};
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::styling::domain::ComputedStyle;
use crate::shared::config::domain::{AxisMargins, FontFamily, FontSize};
use crate::shared::primitives::SizeConstraint;
use crate::shared::primitives::geometry::{Position, Size};
use crate::shared::rendering::ports::text::TextFont;

#[test]
fn test_calculate_layout_styled_text() {
    let mut adapter = TaffyLayoutAdapter::new();

    let render_tree = lay_out(&mut adapter, text_leaf("hello", ComputedStyle::default()));
    assert_eq!(render_tree.rect().width(), 50);
    assert_eq!(render_tree.rect().height(), 20);
}

#[test]
fn test_leaf_padding_grows_the_box_instead_of_eating_the_glyph() {
    // Issue #9: `.day { padding: 4px 2px }` hugged the glyph because the
    // leaf's size was the bare glyph size, so border-box padding came
    // out of it. 5 glyphs * 10 + 2px each side, 20 + 4px each end.
    let mut style = ComputedStyle::default();
    style.set_padding(BoxMargin::new(
        AxisMargins::new(4.0, 4.0),
        AxisMargins::new(2.0, 2.0),
    ));
    let mut adapter = TaffyLayoutAdapter::new();

    let tree = lay_out(&mut adapter, text_leaf("hello", style));

    assert_eq!(tree.rect().width(), 54);
    assert_eq!(tree.rect().height(), 28);
}

/// Sizes text from its font: ten pixels per character of the family name plus one per
/// character of text wide, and the font size tall.
struct FontMeasurer;
impl TextMeasurer for FontMeasurer {
    fn measure(&mut self, text: &str, font: TextFont<'_>) -> Size {
        let family = font.family().map_or(0, |family| family.as_str().len());
        let width = family.saturating_mul(10).saturating_add(text.len());
        let height = font.size().map_or(0.0, FontSize::value);
        Size::new(
            u32::try_from(width).unwrap(),
            crate::utils::f32_to_u32(height),
        )
    }
}

#[test]
fn test_text_leaf_is_measured_in_its_own_font_family_and_size() {
    let mut style = ComputedStyle::default();
    style.set_font_family(FontFamily::new("mono".to_string()));
    style.set_font_size(FontSize::new(17.0));
    let root = flex_node(
        root_path(),
        vec![text_leaf("abc", style)],
        ComputedStyle::default_for_flex(),
    );
    let mut adapter = TaffyLayoutAdapter::new();

    let tree = adapter
        .calculate_layout(
            LayoutRoot::new(root, Position::new(7, 3), SizeConstraint::none()),
            &mut FontMeasurer,
        )
        .unwrap();

    let children = flex_children(tree);
    let leaf = children[0].rect();
    assert_eq!((leaf.width(), leaf.height()), (43, 17));
    assert_eq!((leaf.x(), leaf.y()), (7, 3));
}
