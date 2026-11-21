use super::lifecycle::create_test_event_loop_and_channel;
use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::layout_engine::domain::{DisplayCommand, FloatingKind, ShowFloating};
use crate::features::module_runtime::domain::{LayoutContext, RenderOutcome, RenderPipeline};
use crate::features::module_runtime::test_support::{
    MockCanvasFactory, MockStyleResolver, MockVdomDiff, TestModulePort,
};
use crate::features::vdom::domain::{
    InteractionContext, ModuleParams, NodePath, NodeRef, PanelSpec, PopupSpec, SurfaceSpace, VNode,
};
use crate::shared::primitives::geometry::Scale;
use crate::shared::primitives::{
    ModuleName, ModuleOptions, MonitorFocus, MonitorId, SizeConstraint,
};
use std::collections::HashMap;

/// Renders `bar` with no child size reported, under `interaction`.
fn render(bar: VNode, interaction: Option<InteractionContext>) -> RenderOutcome {
    let mut pipeline = RenderPipeline::new();
    let port = TestModulePort::new(bar);
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
        current_constraint: SizeConstraint::none(),
        current_child_sizes: None,
        interaction_context: interaction,
        canvas_factory: &mut factory,
        layout_engine: &mut engine,
        popup_layout_engine: &mut popup_engine,
        panel_layout_engine: &mut panel_engine,
        tooltip_layout_engine: &mut tooltip_engine,
    };
    pipeline
        .process_monitor(&MonitorId::new("DP-1"), &port, &MockVdomDiff, ctx)
        .expect("render outcome")
}

fn bar() -> VNode {
    VNode::new_rect(None, None, None, None, None)
}

/// Floating content embedding a module that has not reported its size yet;
/// the rect gives it a size, so only the hold keeps it closed.
fn unsettled() -> VNode {
    let child = VNode::new_module(
        ModuleParams::new(ModuleName::new("calendar"), None, ModuleOptions::default()),
        None,
        None,
        None,
        None,
        None,
    );
    VNode::new_flex(vec![child, bar()], None, None, None, None, None)
}

fn render_with_popup(content: VNode) -> RenderOutcome {
    render(bar().with_popup(PopupSpec::new(Box::new(content))), None)
}

fn render_with_panel(content: VNode) -> RenderOutcome {
    render(bar().with_panel(PanelSpec::new(Box::new(content))), None)
}

/// Renders a bar carrying `content` as its tooltip, with the bar hovered.
fn render_with_tooltip(content: VNode) -> RenderOutcome {
    let mut hovered = bar();
    hovered.tooltip = Some(Box::new(content));
    let on_bar = NodeRef::new(Vec::new(), NodePath::root_in(SurfaceSpace::Bar));
    render(
        hovered,
        Some(InteractionContext::new(
            Some(on_bar),
            None,
            None,
            MonitorFocus::Unfocused,
        )),
    )
}

/// The dispatcher sends nothing for a held `kind`, and shows it once the
/// same kind comes with plain content.
fn assert_held_then_shown(render_kind: fn(VNode) -> RenderOutcome, kind: &FloatingKind) {
    let (mut event_loop, _id, display_rx) = create_test_event_loop_and_channel();
    let mon = MonitorId::new("DP-1");

    event_loop.dispatch_render_outcome(&mon, &render_kind(unsettled()), Scale::new(1.0));
    assert!(display_rx.try_recv().is_err());

    event_loop.dispatch_render_outcome(&mon, &render_kind(bar()), Scale::new(1.0));
    let shown = display_rx.try_recv();
    assert!(
        matches!(&shown, Ok(DisplayCommand::ShowFloatingSurface(ShowFloating { kind: sent, .. })) if sent == kind),
        "expected {kind:?} shown, got {shown:?}"
    );
    assert!(display_rx.try_recv().is_err());
}

/// A popup whose embedded child has not reported its size yet is held: the
/// dispatcher leaves the surface closed rather than show a wrong-width frame,
/// while the same popup with plain content is shown at once.
#[test]
fn test_event_loop_does_not_show_a_held_popup() {
    let target = crate::features::layout_engine::domain::PopupTarget::new(
        crate::shared::primitives::ModuleId::new(42),
        MonitorId::new("DP-1"),
    );
    assert_held_then_shown(render_with_popup, &FloatingKind::Popup(target));
}

/// The panel reads its own hold, not the popup's or the tooltip's.
#[test]
fn test_event_loop_does_not_show_a_held_panel() {
    let target = crate::features::layout_engine::domain::PopupTarget::new(
        crate::shared::primitives::ModuleId::new(42),
        MonitorId::new("DP-1"),
    );
    assert_held_then_shown(render_with_panel, &FloatingKind::Panel(target));
}

/// The tooltip reads its own hold, not the popup's or the panel's.
#[test]
fn test_event_loop_does_not_show_a_held_tooltip() {
    assert_held_then_shown(render_with_tooltip, &FloatingKind::Tooltip);
}
