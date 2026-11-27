//! Grid tracks with a gap, and `calc()` widths resolved against the parent's real size.
use super::support::{
    child_path, flex_children, flex_node, grid_children, grid_node, lay_out, rect_node, root_path,
};
use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::layout_engine::domain::{Gap, StyledNode};
use crate::features::styling::domain::{ComputedStyle, CssLength, GridTrack};

/// A rect child at `index` that asks for the full width of its cell.
fn full_width_rect(index: usize) -> StyledNode {
    let mut style = ComputedStyle::default();
    style.set_width(CssLength::Percent(100.0));
    rect_node(child_path(index), style)
}

#[test]
fn test_calculate_layout_grid() {
    let mut adapter = TaffyLayoutAdapter::new();

    let mut grid_style = ComputedStyle::default_for_grid();
    grid_style.set_grid_template_columns(vec![GridTrack::Pixels(50.0), GridTrack::Pixels(100.0)]);
    grid_style.set_column_gap(Gap::new(10.0));
    let grid = grid_node(vec![full_width_rect(0), full_width_rect(1)], grid_style);

    let children = grid_children(lay_out(&mut adapter, grid));
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].rect().x(), 0);
    assert_eq!(children[0].rect().width(), 50);
    assert_eq!(children[1].rect().x(), 60); // 50 + 10 gap
    assert_eq!(children[1].rect().width(), 100);
}

#[test]
fn test_calc_percent_resolves_against_real_parent_width_second_pass() {
    // width: calc(50% - 20px) on a child of a 200px-wide parent — this
    // can only resolve correctly once the parent's actual layout size
    // is known, proving the two-pass mechanism (not just the style
    // cascade) actually runs.
    let mut adapter = TaffyLayoutAdapter::new();

    let mut child_style = ComputedStyle::default();
    child_style.set_width(CssLength::Calc {
        percent: 50.0,
        px: -20.0,
    });
    child_style.set_height(CssLength::Pixels(10.0));
    let child = rect_node(child_path(0), child_style);

    let mut root_style = ComputedStyle::default();
    root_style.set_width(CssLength::Pixels(200.0));
    root_style.set_height(CssLength::Pixels(10.0));
    let root = flex_node(root_path(), vec![child], root_style);

    let children = flex_children(lay_out(&mut adapter, root));
    // 200px * 50% - 20px = 80px.
    assert_eq!(children[0].rect().width(), 80);
}
