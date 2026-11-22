use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::module_runtime::domain::RenderPipeline;
use crate::features::module_runtime::test_support::{
    PipelineFixture, TestModulePort, process_monitor,
};
use crate::features::vdom::domain::{ElementSelectors, Interactions, VNode};
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::{MonitorId, SizeConstraint};

/// Issue #9: an embedded module is laid out at the width its host box
/// resolved to, but reports its *intrinsic* size — the embedder sizes the
/// host from that, so it must not depend on the width it chose — plus the
/// width it settled at, which gates the floating surface.
#[test]
fn test_pinned_child_reports_intrinsic_size_and_settled_width() {
    let mut pipeline = RenderPipeline::new();
    let mon = MonitorId::new("DP-1");
    let port = TestModulePort::new(VNode::new_rect(
        ElementSelectors::default(),
        Interactions::default(),
    ));
    let mut fixture = PipelineFixture::<TaffyLayoutAdapter>::default();
    let mut ctx = fixture.ctx(&mon);
    ctx.current_constraint = SizeConstraint::new(Some(280), None);
    let outcome = process_monitor(&mut pipeline, &port, ctx).expect("render outcome");

    // Painted at the host width ...
    assert_eq!(outcome.render_tree().rect().width(), 280);
    // ... but reported by what it wants on its own.
    let change = outcome.size_change().expect("first report");
    assert_eq!(change.new_size(), Size::new(10, 10));
    assert_eq!(change.settled_width(), Some(280));
}
