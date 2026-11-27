//! The taffy style a styled node is laid out with: its display, and its grid container and grid
//! item properties.
use super::super::grid_converter::{
    grid_auto_flow_to_taffy, grid_line_placement_to_taffy, grid_track_to_template_component,
    grid_track_to_track_sizing_function,
};
use super::super::style_builder::node_to_style;
use crate::features::layout_engine::domain::{
    AlignItems, JustifyContent, NodePath, StyledNode, SurfaceSpace, TextMeasurer,
};
use crate::features::styling::domain::{
    ComputedStyle, CssLength, DisplayMode, GridAutoFlow, GridLinePlacement, GridPlacement,
    GridTrack,
};
use crate::shared::primitives::ModuleKey;
use crate::shared::primitives::geometry::Size;
use crate::shared::rendering::ports::text::TextFont;
use taffy::style::{Display, Style};

/// Measures nothing but modules, which all report 150x28.
struct ModuleMeasurer;

impl TextMeasurer for ModuleMeasurer {
    fn measure(&mut self, _text: &str, _font: TextFont<'_>) -> Size {
        Size::new(0, 0)
    }

    fn measure_module(&self, _key: &ModuleKey) -> Option<Size> {
        Some(Size::new(150, 28))
    }
}

/// A rect styled with `style`.
pub(super) fn rect(style: ComputedStyle) -> StyledNode {
    StyledNode::Rect {
        path: NodePath::root_in(SurfaceSpace::Bar),
        node_key: None,
        style,
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    }
}

/// A grid container with no children, styled with `style`.
fn grid(style: ComputedStyle) -> StyledNode {
    StyledNode::Grid {
        path: NodePath::root_in(SurfaceSpace::Bar),
        node_key: None,
        children: vec![],
        style,
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    }
}

pub(super) fn taffy_style(node: &StyledNode) -> Style {
    node_to_style(node, &mut ModuleMeasurer)
}

pub(super) fn px(value: f32) -> CssLength {
    CssLength::Pixels(value)
}

#[test]
fn a_grid_container_takes_its_tracks() {
    let columns = vec![GridTrack::Fraction(1.0), GridTrack::Pixels(40.0)];
    let mut style = ComputedStyle::default();
    style.set_grid_template_columns(columns.clone());
    style.set_grid_template_rows(vec![GridTrack::Pixels(20.0)]);
    style.set_grid_auto_columns(vec![GridTrack::Auto]);
    style.set_grid_auto_rows(vec![GridTrack::MinContent]);
    let taffy = taffy_style(&grid(style));

    assert_eq!(taffy.display, Display::Grid);
    let template = |track: &GridTrack| grid_track_to_template_component(track);
    let explicit: Vec<_> = columns.iter().map(template).collect();
    assert_eq!(taffy.grid_template_columns, explicit);
    assert_eq!(
        taffy.grid_template_rows,
        [template(&GridTrack::Pixels(20.0))]
    );
    let implicit = |track: GridTrack| vec![grid_track_to_track_sizing_function(&track)];
    assert_eq!(taffy.grid_auto_columns, implicit(GridTrack::Auto));
    assert_eq!(taffy.grid_auto_rows, implicit(GridTrack::MinContent));
}

#[test]
fn a_grid_container_takes_its_flow_and_alignment() {
    let mut style = ComputedStyle::default();
    style.set_grid_auto_flow(GridAutoFlow::Column);
    style.set_justify_items(AlignItems::Center);
    style.set_align_content(JustifyContent::End);
    let taffy = taffy_style(&grid(style));

    let flow = grid_auto_flow_to_taffy(GridAutoFlow::Column);
    assert_eq!(taffy.grid_auto_flow, flow);
    assert_eq!(taffy.justify_items, Some(AlignItems::Center.into()));
    assert_eq!(taffy.align_content, Some(JustifyContent::End.into()));
}

/// A node's own display wins over its kind; without one a grid node is a grid and any other a
/// flex container; `display: none` hides it.
#[test]
fn the_display_comes_from_the_style_else_the_node_kind() {
    let display = |node: StyledNode| taffy_style(&node).display;
    let mut flex = ComputedStyle::default();
    flex.set_display(DisplayMode::Flex);
    assert_eq!(display(grid(flex)), Display::Flex);
    let mut hidden = ComputedStyle::default();
    hidden.set_display(DisplayMode::None);
    assert_eq!(display(rect(hidden)), Display::None);
    let mut as_grid = ComputedStyle::default();
    as_grid.set_display(DisplayMode::Grid);
    assert_eq!(display(rect(as_grid)), Display::Grid);
    assert_eq!(display(grid(ComputedStyle::default())), Display::Grid);
    assert_eq!(display(rect(ComputedStyle::default())), Display::Flex);
}

#[test]
fn a_grid_item_takes_its_lines_and_self_justification() {
    let column = GridLinePlacement::new(GridPlacement::Line(2), GridPlacement::Span(2));
    let row = GridLinePlacement::new(GridPlacement::Span(3), GridPlacement::Auto);
    let mut style = ComputedStyle::default();
    style.set_grid_column(column);
    style.set_grid_row(row);
    style.set_justify_self(AlignItems::End);
    let taffy = taffy_style(&rect(style));

    assert_eq!(taffy.grid_column, grid_line_placement_to_taffy(column));
    assert_eq!(taffy.grid_row, grid_line_placement_to_taffy(row));
    assert_eq!(taffy.justify_self, Some(AlignItems::End.into()));
}
