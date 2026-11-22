use super::lifecycle::create_test_event_loop_and_channel;
use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::layout_engine::domain::{DisplayCommand, FloatingKind, ShowFloating};
use crate::features::module_runtime::domain::{RenderOutcome, RenderPipeline};
use crate::features::module_runtime::test_support::{
    PipelineFixture, TestModulePort, process_monitor,
};
use crate::features::vdom::domain::{ElementSelectors, Interactions};
use crate::features::vdom::domain::{
    InteractedNodes, InteractionContext, ModuleParams, NodePath, NodeRef, PanelSpec, PopupSpec,
    SurfaceSpace, VNode,
};
use crate::shared::primitives::{ModuleName, ModuleOptions, MonitorFocus, MonitorId};

/// Renders `bar` with no child size reported, under `interaction`.
fn render(bar: VNode, interaction: Option<InteractionContext>) -> RenderOutcome {
    let mut fixture = PipelineFixture::<TaffyLayoutAdapter>::default();
    let monitor = MonitorId::new("DP-1");
    let mut ctx = fixture.ctx(&monitor);
    ctx.interaction_context = interaction;
    let port = TestModulePort::new(bar);
    process_monitor(&mut RenderPipeline::new(), &port, ctx).expect("render outcome")
}

fn bar() -> VNode {
    VNode::new_rect(ElementSelectors::default(), Interactions::default())
}

/// Floating content embedding a module that has not reported its size yet;
/// the rect gives it a size, so only the hold keeps it closed.
fn unsettled() -> VNode {
    let child = VNode::new_module(
        ModuleParams::new(ModuleName::new("calendar"), None, ModuleOptions::default()),
        ElementSelectors::default(),
        Interactions::default(),
    );
    VNode::new_flex(
        vec![child, bar()],
        ElementSelectors::default(),
        Interactions::default(),
    )
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
            InteractedNodes::new(Some(on_bar), None, None),
            MonitorFocus::Unfocused,
        )),
    )
}

/// The dispatcher sends nothing for a held `kind`, and shows it once the
/// same kind comes with plain content.
fn assert_held_then_shown(render_kind: fn(VNode) -> RenderOutcome, kind: &FloatingKind) {
    let (mut event_loop, _id, display_rx) = create_test_event_loop_and_channel();
    let mon = MonitorId::new("DP-1");

    event_loop.dispatch_render_outcome(&mon, &render_kind(unsettled()));
    assert!(display_rx.try_recv().is_err());

    event_loop.dispatch_render_outcome(&mon, &render_kind(bar()));
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

/// A held popup leaves the surface showing what it showed, sending nothing, and keeps it as what
/// is shown: the same plain popup afterwards is not sent again.
#[test]
fn a_held_popup_keeps_what_it_showed() {
    let (mut event_loop, _id, display_rx) = create_test_event_loop_and_channel();
    let mon = MonitorId::new("DP-1");
    event_loop.dispatch_render_outcome(&mon, &render_with_popup(bar()));
    assert_eq!(display_rx.try_iter().count(), 1);

    event_loop.dispatch_render_outcome(&mon, &render_with_popup(unsettled()));
    event_loop.dispatch_render_outcome(&mon, &render_with_popup(bar()));
    assert!(display_rx.try_recv().is_err());
}
