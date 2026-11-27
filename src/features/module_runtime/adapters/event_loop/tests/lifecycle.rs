use super::fixture::{ChannelEventLoop, MODULE, empty_hub, wired_event_loop};
use crate::features::layout_engine::domain::{
    DisplayCommand, FloatingKind, NodePath, PopupTarget, RenderNode, SurfaceSpace,
};
use crate::features::module_runtime::domain::render_pipeline::{EmbedReport, PipelineLayout};
use crate::features::module_runtime::domain::{FloatingLayouts, PopupRenderLayout, RenderOutcome};
use crate::features::module_runtime::test_support::TestModulePort;
use crate::features::styling::domain::ComputedStyle;
use crate::features::vdom::domain::{ElementSelectors, Interactions, TextContent, VNode};
use crate::shared::primitives::geometry::{Position, Rect, Scale, Size};
use crate::shared::primitives::{ModuleId, MonitorId};
use std::sync::mpsc::Receiver;

/// An event loop for module 42 rendering a plain rect, with the display commands it sends.
pub(super) fn create_test_event_loop_and_channel()
-> (ChannelEventLoop, ModuleId, Receiver<DisplayCommand>) {
    let rect = VNode::new_rect(ElementSelectors::default(), Interactions::default());
    let (event_loop, wiring) = wired_event_loop(TestModulePort::new(rect), empty_hub());
    (event_loop, MODULE, wiring.sent.display)
}

pub(super) fn make_test_rect_node() -> RenderNode {
    RenderNode::Rect {
        path: NodePath::root_in(SurfaceSpace::Bar),
        node_key: None,
        rect: Rect::new(Position::new(0, 0), Size::new(50, 20)),
        style: ComputedStyle::default(),
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    }
}

/// Popup text of `size` at the origin, as the pipeline's layout pass would have produced it.
pub(super) fn popup_text(size: Size) -> RenderNode {
    RenderNode::Text {
        path: NodePath::root_in(SurfaceSpace::Bar),
        node_key: None,
        rect: Rect::new(Position::new(0, 0), size),
        text: TextContent::new("popup".to_string()),
        style: ComputedStyle::default(),
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    }
}

/// An outcome at scale 1 with `popup` open, or with nothing open.
pub(super) fn outcome_with(popup: Option<PopupRenderLayout>) -> RenderOutcome {
    RenderOutcome::new(
        PipelineLayout::new(
            make_test_rect_node(),
            FloatingLayouts::new(popup, None, None),
            EmbedReport::new(None, vec![]),
        ),
        None,
        Scale::new(1.0),
    )
}

#[test]
fn test_event_loop_popup_floating_surface_lifecycle() {
    let (mut event_loop, id, display_rx) = create_test_event_loop_and_channel();
    let mon = MonitorId::new("DP-1");
    let popup_content = popup_text(Size::new(30, 12));
    let expected_anchor_rect = Rect::new(Position::new(0, 0), Size::new(50, 20));
    let popup_layout = PopupRenderLayout::new(expected_anchor_rect, popup_content.clone(), None);
    event_loop.dispatch_render_outcome(&mon, &outcome_with(Some(popup_layout)));

    let shown = display_rx.try_recv();
    let Ok(DisplayCommand::ShowFloatingSurface(show)) = shown else {
        panic!("Expected ShowFloatingSurface, got {shown:?}");
    };
    assert_eq!((show.offset, show.parent), (None, None));
    assert_eq!(
        show.kind,
        FloatingKind::Popup(PopupTarget::new(id, mon.clone()))
    );
    assert_eq!(show.monitor_id, Some(mon.clone()));
    assert_eq!(show.anchor_rect, Some(expected_anchor_rect));
    // The buffer was painted from `popup_content`'s own rect — proof that what was dispatched
    // came from that layout, not a stale or unrelated one.
    assert_eq!(show.logical_size, *popup_content.rect().size());

    event_loop.dispatch_render_outcome(&mon, &outcome_with(None));
    let cmd2 = display_rx
        .try_recv()
        .expect("Should have sent HideFloatingSurface");
    let hidden = FloatingKind::Popup(PopupTarget::new(id, mon));
    assert_eq!(cmd2, DisplayCommand::HideFloatingSurface { kind: hidden });
}
