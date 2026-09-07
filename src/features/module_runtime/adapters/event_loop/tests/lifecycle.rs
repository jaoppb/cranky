use crate::features::module_runtime::adapters::event_loop::runner::EventLoop;
use crate::features::module_runtime::application::ModuleContext;
use crate::features::module_runtime::domain::{PointerHandler, RenderPipeline};
use crate::features::module_runtime::test_support::{
    ChannelDisplaySender, MockCanvasFactory, MockLayoutSender, MockSurfaceManager, MockUiSender,
    TestModulePort,
};
use crate::features::styling::adapters::fs_loader::CompositeStyleResolver;
use crate::features::vdom::adapters::DefaultVdomDiffAdapter;
use crate::features::vdom::domain::VNode;
use crate::shared::config::domain::Config;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::geometry::{Position, Rect, Size};
use crate::shared::primitives::ModuleId;
use crate::shared::primitives::MonitorId;
use std::collections::HashMap;
use std::sync::Arc;

type TestEventLoop = EventLoop<
    MockCanvasFactory,
    MockLayoutSender,
    ChannelDisplaySender,
    MockUiSender,
>;

fn create_test_event_loop_and_channel() -> (
    TestEventLoop,
    ModuleId,
    std::sync::mpsc::Receiver<crate::features::layout_engine::domain::DisplayCommand>,
) {
    let id = ModuleId::new(42);
    let hub = Arc::new(SignalHub::new(Config::default()));
    let sm = Arc::new(MockSurfaceManager);
    let layout_sender = Arc::new(MockLayoutSender);
    let (display_tx, display_rx) = std::sync::mpsc::channel();
    let display_sender = Arc::new(ChannelDisplaySender::new(display_tx));
    let ui_sender = Arc::new(MockUiSender);
    let (_tx, rx) = tokio::sync::watch::channel(HashMap::new());
    let ctx = ModuleContext::new(id, hub, sm, layout_sender, display_sender, ui_sender, rx);

    let event_loop = EventLoop::new(
        Box::new(TestModulePort::new(VNode::new_rect(
            None, None, None, None, None,
        ))),
        ctx,
        PointerHandler::new(),
        RenderPipeline::new(),
        MockCanvasFactory,
        Arc::new(CompositeStyleResolver::new(vec![])),
        Arc::new(DefaultVdomDiffAdapter::new()),
    );

    (event_loop, id, display_rx)
}

fn make_test_rect_node(
    popup: Option<crate::features::layout_engine::domain::StyledPopup>,
) -> crate::features::layout_engine::domain::RenderNode {
    crate::features::layout_engine::domain::RenderNode::Rect {
        path: crate::features::layout_engine::domain::NodePath::root(),
        rect: Rect::new(Position::new(0, 0), Size::new(50, 20)),
        style: crate::features::styling::domain::ComputedStyle::default(),
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup,
        panel: None,
    }
}

#[test]
fn test_event_loop_popup_floating_surface_lifecycle() {
    use crate::features::layout_engine::domain::{DisplayCommand, FloatingKind, StyledNode, StyledPopup};
    use crate::features::styling::domain::ComputedStyle;

    let (mut event_loop, id, display_rx) = create_test_event_loop_and_channel();
    let mon = MonitorId::new("DP-1");

    let popup_styled = StyledNode::Text {
        path: crate::features::layout_engine::domain::NodePath::root(),
        text: crate::features::vdom::domain::TextContent::new("popup".to_string()),
        style: ComputedStyle::default(),
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    };
    let styled_popup = StyledPopup::new(
        Box::new(popup_styled.clone()),
        crate::features::vdom::domain::AnchorDirection::default(),
        None,
        true,
    );
    let outcome1 = crate::features::module_runtime::domain::RenderOutcome::new(
        None,
        vec![],
        make_test_rect_node(Some(styled_popup)),
        None,
    );
    event_loop.dispatch_render_outcome(&mon, &outcome1);

    let cmd1 = display_rx
        .try_recv()
        .expect("Should have sent ShowFloatingSurface");
    match cmd1 {
        DisplayCommand::ShowFloatingSurface {
            kind,
            monitor_id,
            anchor_rect,
            layout,
            offset,
        } => {
            assert_eq!(offset, None);
            assert_eq!(
                kind,
                FloatingKind::Popup(crate::features::layout_engine::domain::PopupTarget::new(
                    id,
                    mon.clone(),
                ),)
            );
            assert_eq!(monitor_id, Some(mon.clone()));
            assert_eq!(
                anchor_rect,
                Some(Rect::new(Position::new(0, 0), Size::new(50, 20)))
            );
            assert_eq!(*layout, popup_styled);
        }
        _ => panic!("Expected ShowFloatingSurface, got {cmd1:?}"),
    }

    let outcome2 = crate::features::module_runtime::domain::RenderOutcome::new(
        None,
        vec![],
        make_test_rect_node(None),
        None,
    );
    event_loop.dispatch_render_outcome(&mon, &outcome2);

    let cmd2 = display_rx
        .try_recv()
        .expect("Should have sent HideFloatingSurface");
    match cmd2 {
        DisplayCommand::HideFloatingSurface { kind } => {
            assert_eq!(
                kind,
                FloatingKind::Popup(crate::features::layout_engine::domain::PopupTarget::new(
                    id, mon,
                ),)
            );
        }
        _ => panic!("Expected HideFloatingSurface, got {cmd2:?}"),
    }
}
