//! A `calc(<percent>% + <px>px)` on each length field resolves against its parent's laid-out
//! size, along the axis of that field; a root's has no parent and keeps its absolute part.
use super::super::TaffyLayoutAdapter;
use crate::features::layout_engine::domain::{
    LayoutRoot, NodePath, RenderNode, StyledNode, SurfaceSpace, TextMeasurer,
};
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::styling::domain::{ComputedStyle, CssLength};
use crate::shared::primitives::geometry::{Position, Rect, Size};
use crate::shared::primitives::{ModuleKey, SizeConstraint};
use crate::shared::rendering::ports::text::TextFont;

struct NoText;

impl TextMeasurer for NoText {
    fn measure(&mut self, _text: &str, _font: TextFont<'_>) -> Size {
        Size::new(0, 0)
    }

    fn measure_module(&self, _key: &ModuleKey) -> Option<Size> {
        None
    }
}

const fn calc(percent: f32, px: f32) -> CssLength {
    CssLength::Calc { percent, px }
}

const fn px(value: f32) -> CssLength {
    CssLength::Pixels(value)
}

fn flex(path: NodePath, style: ComputedStyle, children: Vec<StyledNode>) -> StyledNode {
    StyledNode::Flex {
        path,
        node_key: None,
        children,
        style,
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    }
}

/// Lays `root` out unconstrained.
fn lay_out(root: StyledNode) -> RenderNode {
    let root = LayoutRoot::new(root, Position::new(0, 0), SizeConstraint::none());
    TaffyLayoutAdapter::new()
        .calculate_layout(root, &mut NoText)
        .unwrap()
}

/// The box of a childless container styled by `child_style`, in a 200x100 row.
fn child_rect(child_style: impl FnOnce(&mut ComputedStyle)) -> Rect {
    let mut style = ComputedStyle::default();
    child_style(&mut style);
    let child = flex(NodePath::new(SurfaceSpace::Bar, vec![0]), style, vec![]);
    let mut parent = ComputedStyle::default();
    parent.set_width(px(200.0));
    parent.set_height(px(100.0));
    let tree = lay_out(flex(
        NodePath::root_in(SurfaceSpace::Bar),
        parent,
        vec![child],
    ));
    tree.children()[0].rect()
}

#[test]
fn a_height_resolves_against_the_parent_height() {
    assert_eq!(
        child_rect(|style| style.set_height(calc(50.0, 2.0))).height(),
        52
    );
}

#[test]
fn width_bounds_resolve_against_the_parent_width() {
    let at_least = child_rect(|style| {
        style.set_width(px(10.0));
        style.set_min_width(calc(25.0, 0.0));
    });
    let at_most = child_rect(|style| {
        style.set_width(px(150.0));
        style.set_max_width(calc(10.0, 5.0));
    });
    assert_eq!((at_least.width(), at_most.width()), (50, 25));
}

#[test]
fn height_bounds_resolve_against_the_parent_height() {
    let at_least = child_rect(|style| {
        style.set_height(px(10.0));
        style.set_min_height(calc(30.0, 0.0));
    });
    let at_most = child_rect(|style| {
        style.set_height(px(90.0));
        style.set_max_height(calc(40.0, 1.0));
    });
    assert_eq!((at_least.height(), at_most.height()), (30, 41));
}

#[test]
fn a_flex_basis_resolves_against_the_parent_width() {
    assert_eq!(
        child_rect(|style| style.set_flex_basis(calc(30.0, 0.0))).width(),
        60
    );
}

/// A root has no parent to resolve against, so its `calc()` keeps its absolute part.
#[test]
fn a_root_calc_keeps_its_absolute_part() {
    let mut style = ComputedStyle::default();
    style.set_width(calc(50.0, 70.0));
    style.set_height(px(10.0));
    let tree = lay_out(flex(NodePath::root_in(SurfaceSpace::Bar), style, vec![]));
    assert_eq!(tree.rect().width(), 70);
}
