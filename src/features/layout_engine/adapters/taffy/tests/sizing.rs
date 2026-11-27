//! The taffy style a styled node is laid out with: its size bounds, flex item properties and
//! gaps, and the size an embedded module reports.
use super::style::{px, rect, taffy_style};
use crate::features::layout_engine::domain::{AlignItems, Gap, NodePath, StyledNode, SurfaceSpace};
use crate::features::styling::domain::{ComputedStyle, FlexGrow, FlexShrink};
use crate::shared::primitives::{ModuleKey, ModuleName, ModuleOptions};
use taffy::style::{Dimension, LengthPercentage};

#[test]
fn a_node_takes_its_size_and_its_bounds() {
    let mut style = ComputedStyle::default();
    style.set_width(px(31.0));
    style.set_height(px(32.0));
    style.set_min_width(px(33.0));
    style.set_max_width(px(34.0));
    style.set_min_height(px(35.0));
    style.set_max_height(px(36.0));
    let taffy = taffy_style(&rect(style));

    let sizes = [taffy.size, taffy.min_size, taffy.max_size];
    let lengths = sizes.map(|size| (size.width, size.height));
    assert_eq!(
        lengths,
        [(31.0, 32.0), (33.0, 35.0), (34.0, 36.0)]
            .map(|(width, height)| (Dimension::length(width), Dimension::length(height)))
    );
}

#[test]
fn a_flex_item_takes_its_grow_shrink_basis_and_self_alignment() {
    let mut style = ComputedStyle::default();
    style.set_flex_grow(FlexGrow::new(2.0).unwrap());
    style.set_flex_shrink(FlexShrink::new(0.5).unwrap());
    style.set_flex_basis(px(30.0));
    style.set_align_self(AlignItems::Center);
    let taffy = taffy_style(&rect(style));

    assert_eq!((taffy.flex_grow, taffy.flex_shrink), (2.0, 0.5));
    assert_eq!(taffy.flex_basis, Dimension::length(30.0));
    assert_eq!(taffy.align_self, Some(AlignItems::Center.into()));
}

/// Each axis takes its own gap over the shorthand, and no gap at all is zero.
#[test]
fn each_gap_axis_takes_its_own_gap_else_the_shorthand() {
    let mut style = ComputedStyle::default();
    style.set_gap(Gap::new(6.0));
    style.set_column_gap(Gap::new(4.0));
    let gaps = taffy_style(&rect(style)).gap;
    assert_eq!(
        (gaps.width, gaps.height),
        (LengthPercentage::length(4.0), LengthPercentage::length(6.0))
    );

    let mut rows_only = ComputedStyle::default();
    rows_only.set_row_gap(Gap::new(5.0));
    let gaps = taffy_style(&rect(rows_only)).gap;
    assert_eq!(
        (gaps.width, gaps.height),
        (LengthPercentage::length(0.0), LengthPercentage::length(5.0))
    );
}

/// A module embedded with `style`.
fn module(style: ComputedStyle) -> StyledNode {
    StyledNode::Module {
        path: NodePath::root_in(SurfaceSpace::Bar),
        node_key: None,
        key: ModuleKey::from_name(ModuleName::new("clock")),
        options: ModuleOptions::default(),
        style,
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    }
}

/// An embedded module is sized to what it reported (150x28) on each axis its style leaves
/// unset.
#[test]
fn an_embedded_module_takes_its_reported_size_where_its_style_sets_none() {
    let size_with = |change: fn(&mut ComputedStyle)| {
        let mut style = ComputedStyle::default();
        change(&mut style);
        let size = taffy_style(&module(style)).size;
        (size.width, size.height)
    };
    let length = Dimension::length;
    let wide = size_with(|style| style.set_width(px(80.0)));
    assert_eq!(wide, (length(80.0), length(28.0)));
    let tall = size_with(|style| style.set_height(px(40.0)));
    assert_eq!(tall, (length(150.0), length(40.0)));
}
