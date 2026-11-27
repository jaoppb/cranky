//! Embedded module nodes: explicit size, measured size, nested layouts and the embedder's
//! own min-width.
use super::support::{
    child_path, flex_node, lay_out, lay_out_root, module_node, root_path, unpinned,
};
use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::layout_engine::domain::{BoxMargin, LayoutRoot, TextMeasurer};
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::styling::domain::{ComputedStyle, CssLength};
use crate::shared::config::domain::AxisMargins;
use crate::shared::primitives::geometry::{Position, Size};
use crate::shared::primitives::{ModuleKey, SizeConstraint};
use crate::shared::rendering::ports::text::TextFont;

#[test]
fn test_calculate_layout_styled_module_with_custom_size() {
    let mut adapter = TaffyLayoutAdapter::new();

    let mut style = ComputedStyle::default();
    style.set_width(CssLength::Pixels(120.0));
    style.set_height(CssLength::Pixels(30.0));
    let node = module_node(root_path(), "custom_mod", style);

    let render_tree = lay_out_root(
        &mut adapter,
        LayoutRoot::new(node, Position::new(10, 5), SizeConstraint::none()),
    );
    assert_eq!(render_tree.rect().x(), 10);
    assert_eq!(render_tree.rect().y(), 5);
    assert_eq!(render_tree.rect().width(), 120);
    assert_eq!(render_tree.rect().height(), 30);
}

#[test]
fn test_calculate_layout_styled_module_with_measured_size() {
    let mut adapter = TaffyLayoutAdapter::new();
    let node = module_node(root_path(), "workspace", ComputedStyle::default());

    let render_tree = lay_out(&mut adapter, node);
    assert_eq!(render_tree.rect().width(), 150);
    assert_eq!(render_tree.rect().height(), 28);
}

#[test]
fn test_nested_container_module_collect_layouts() {
    let mut adapter = TaffyLayoutAdapter::new();

    let child1 = module_node(child_path(0), "workspace", ComputedStyle::default());
    let mut root_style = ComputedStyle::default();
    root_style.set_padding(BoxMargin::new(
        AxisMargins::new(0.0, 0.0),
        AxisMargins::new(16.0, 16.0),
    ));
    let root = flex_node(root_path(), vec![child1], root_style);

    let render_tree = lay_out(&mut adapter, root);

    let layouts = render_tree.collect_module_layouts();
    assert_eq!(layouts.len(), 1);
    assert_eq!(layouts[0].key().name().as_str(), "workspace");
    assert_eq!(layouts[0].bounds().rect().x(), 16);
}

/// Reports every module at 240x190 and every text at zero size.
struct CalendarMeasurer;
impl TextMeasurer for CalendarMeasurer {
    fn measure(&mut self, _: &str, _font: TextFont<'_>) -> Size {
        Size::new(0, 0)
    }
    fn measure_module(&self, _: &ModuleKey) -> Option<Size> {
        Some(Size::new(240, 190))
    }
}

#[test]
fn test_embedder_min_width_overrides_the_childs_reported_width() {
    // Issue #9 decision: a rule on the embed node (`.calendar {
    // min-width: 280px }`) wins over the child's own default (reported
    // as 240) and persists — it is a real constraint, not a hint.
    let mut style = ComputedStyle::default();
    style.set_min_width(CssLength::Pixels(280.0));
    let node = module_node(root_path(), "calendar", style);
    let mut adapter = TaffyLayoutAdapter::new();

    let tree = adapter
        .calculate_layout(unpinned(node), &mut CalendarMeasurer)
        .unwrap();

    assert_eq!(tree.rect().width(), 280);
    assert_eq!(tree.rect().height(), 190);
}

/// A module laid out again with a new style is restyled in place, not kept as it was.
#[test]
fn a_module_laid_out_again_with_a_new_style_takes_it() {
    let mut adapter = TaffyLayoutAdapter::new();
    let sized = |width: f32| {
        let mut style = ComputedStyle::default();
        style.set_width(CssLength::Pixels(width));
        module_node(root_path(), "custom_mod", style)
    };
    lay_out(&mut adapter, sized(120.0));
    let again = lay_out(&mut adapter, sized(70.0));
    assert_eq!(again.rect().width(), 70);
}
