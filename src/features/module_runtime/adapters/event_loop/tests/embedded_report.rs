use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::module_runtime::domain::{LayoutContext, RenderPipeline};
use crate::features::module_runtime::test_support::{
    MockCanvasFactory, MockStyleResolver, MockVdomDiff, TestModulePort,
};
use crate::features::vdom::domain::VNode;
use crate::shared::primitives::geometry::{Scale, Size};
use crate::shared::primitives::{MonitorId, SizeConstraint};
use std::collections::HashMap;

/// Issue #9: an embedded module is laid out at the width its host box
/// resolved to, but reports its *intrinsic* size — the embedder sizes the
/// host from that, so it must not depend on the width it chose — plus the
/// width it settled at, which gates the floating surface.
#[test]
fn test_pinned_child_reports_intrinsic_size_and_settled_width() {
    let mut pipeline = RenderPipeline::new();
    let mon = MonitorId::new("DP-1");
    let port = TestModulePort::new(VNode::new_rect(None, None, None, None, None));
    let resolver = MockStyleResolver;
    let mut factory = MockCanvasFactory;
    let mut engine = TaffyLayoutAdapter::new();
    let mut popup_engine = TaffyLayoutAdapter::new();
    let mut panel_engine = TaffyLayoutAdapter::new();
    let mut tooltip_engine = TaffyLayoutAdapter::new();
    let child_errors = HashMap::new();

    let ctx = LayoutContext {
        scale: Scale::new(1.0),
        style_resolver: &resolver,
        child_errors: &child_errors,
        current_bounds: None,
        current_constraint: SizeConstraint::new(Some(280), None),
        current_child_sizes: None,
        interaction_context: None,
        canvas_factory: &mut factory,
        layout_engine: &mut engine,
        popup_layout_engine: &mut popup_engine,
        panel_layout_engine: &mut panel_engine,
        tooltip_layout_engine: &mut tooltip_engine,
    };
    let outcome = pipeline
        .process_monitor(&mon, &port, &MockVdomDiff, ctx)
        .expect("render outcome");

    // Painted at the host width ...
    assert_eq!(outcome.render_tree().rect().width(), 280);
    // ... but reported by what it wants on its own.
    let change = outcome.size_change().expect("first report");
    assert_eq!(change.new_size(), Size::new(10, 10));
    assert_eq!(change.settled_width(), Some(280));
}
