//! A container whose style changes between passes is laid out with the new style.
use super::support::{flex_children, flex_node, lay_out, root_path, text_leaf};
use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::layout_engine::domain::{BoxMargin, StyledNode};
use crate::features::styling::domain::ComputedStyle;
use crate::shared::config::domain::AxisMargins;

/// A root flex container padded `top` and `left`, holding the text `hey`.
fn padded(top: f64, left: f64) -> StyledNode {
    let mut style = ComputedStyle::default_for_flex();
    style.set_padding(BoxMargin::new(
        AxisMargins::new(top, 0.0),
        AxisMargins::new(left, 0.0),
    ));
    flex_node(
        root_path(),
        vec![text_leaf("hey", ComputedStyle::default())],
        style,
    )
}

#[test]
fn test_a_container_whose_style_changed_is_laid_out_with_the_new_style() {
    let mut adapter = TaffyLayoutAdapter::new();
    let first = lay_out(&mut adapter, padded(1.0, 2.0));
    assert_eq!((first.rect().width(), first.rect().height()), (32, 21));

    let second = lay_out(&mut adapter, padded(6.0, 9.0));
    assert_eq!((second.rect().width(), second.rect().height()), (39, 26));
    let children = flex_children(second);
    assert_eq!((children[0].rect().x(), children[0].rect().y()), (9, 6));
}
