//! Size constraints on the root: a pinned axis, dropping the pin, and `fr` tracks filling it.
use super::support::{
    child_path, grid_children, grid_node, lay_out, lay_out_root, pinned_width, text_leaf,
    text_node, unpinned,
};
use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::layout_engine::domain::StyledNode;
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::styling::domain::{ComputedStyle, CssLength, GridTrack};
use crate::shared::primitives::geometry::Size;

#[test]
fn test_size_constraint_pins_one_axis_and_leaves_the_other_intrinsic() {
    // A `width: 100%` root has nothing to resolve against on its own —
    // it needs a definite available space on that axis, which is
    // exactly what an embedded module's SizeConstraint supplies for a
    // pinned axis and withholds for a free one.
    let mut style = ComputedStyle::default();
    style.set_width(CssLength::Percent(100.0));
    let node = || text_leaf("hello", style.clone());

    // Width pinned to 300, height left intrinsic: width resolves the
    // 100% against the pin; height still comes from the measurer, not
    // from the (nonexistent) pin.
    let pinned = lay_out_root(&mut TaffyLayoutAdapter::new(), pinned_width(node(), 300));
    assert_eq!(pinned.rect().width(), 300);
    assert_eq!(pinned.rect().height(), 20);

    // No constraint at all: `100%` has no containing block to resolve
    // against (MaxContent isn't a definite basis), so it resolves to 0
    // — never to whatever a parent's rect happened to be. This is the
    // actual fix: under the old model `available_size` came from
    // `current_bounds`, so a `width: 100%` child always matched
    // whatever slot the parent last assigned, which is exactly the
    // feedback loop decision 10 removes. Now only an explicit
    // `SizeConstraint` pin can produce a definite available space.
    let unconstrained = lay_out(&mut TaffyLayoutAdapter::new(), node());
    assert_eq!(unconstrained.rect().width(), 0);
    assert_eq!(unconstrained.rect().height(), 20);
}

/// A root grid of one `1fr` column holding the text `ab`.
fn one_fr_grid() -> StyledNode {
    let mut style = ComputedStyle::default_for_grid();
    style.set_grid_template_columns(vec![GridTrack::Fraction(1.0)]);
    grid_node(vec![text_leaf("ab", ComputedStyle::default())], style)
}

#[test]
fn test_dropping_a_constraint_unpins_the_cached_root() {
    let mut adapter = TaffyLayoutAdapter::new();
    let pinned = lay_out_root(&mut adapter, pinned_width(one_fr_grid(), 280));
    assert_eq!(pinned.rect().width(), 280);
    // What the embedder measures by is the unpinned size, not the pin.
    assert_eq!(adapter.intrinsic_size(), Some(Size::new(20, 20)));

    let free = lay_out_root(&mut adapter, unpinned(one_fr_grid()));
    assert_eq!(free.rect().width(), 20);
    assert_eq!(adapter.intrinsic_size(), None);
}

#[test]
fn test_fr_columns_fill_a_pinned_root_width() {
    // Issue #9: `repeat(7, 1fr)` collapsed to content-sized columns
    // because the child was never given a definite width. Pinned to
    // 280, seven `1fr` tracks must split it evenly (40px each).
    let cell = |index: usize| text_node(child_path(index), "1", ComputedStyle::default());
    let mut grid_style = ComputedStyle::default_for_grid();
    grid_style.set_grid_template_columns(vec![GridTrack::Fraction(1.0); 7]);
    let grid = grid_node((0..7).map(cell).collect(), grid_style);
    let mut adapter = TaffyLayoutAdapter::new();

    let tree = lay_out_root(&mut adapter, pinned_width(grid, 280));

    assert_eq!(tree.rect().width(), 280);
    let column_offsets: Vec<u32> = grid_children(tree)
        .iter()
        .map(|child| u32::try_from(child.rect().x()).unwrap())
        .collect();
    assert_eq!(column_offsets, vec![0, 40, 80, 120, 160, 200, 240]);
}
