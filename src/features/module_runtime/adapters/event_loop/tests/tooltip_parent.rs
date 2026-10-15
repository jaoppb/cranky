use super::lifecycle::make_test_rect_node;
use crate::features::layout_engine::domain::{DisplayCommand, FloatingKind, PopupTarget};
use crate::features::module_runtime::adapters::event_loop::runner::EventLoop;
use crate::features::module_runtime::application::ModuleContext;
use crate::features::module_runtime::domain::{FloatingLayouts, PointerHandler, RenderPipeline};
use crate::features::module_runtime::test_support::{
    ChannelDisplaySender, MockCanvasFactory, MockLayoutSender, MockSurfaceManager, MockUiSender,
    TestModulePort,
};
use crate::features::styling::adapters::fs_loader::CompositeStyleResolver;
use crate::features::vdom::adapters::DefaultVdomDiffAdapter;
use crate::features::vdom::domain::VNode;
use crate::shared::config::domain::Config;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::geometry::Scale;
use crate::shared::primitives::{LayoutSurface, ModuleId, MonitorId};
use std::collections::HashMap;
use std::sync::Arc;

/// Regression test for Gap B: a module embedded inside an open popup that
/// itself shows a tooltip must dispatch that tooltip with the real floating
/// parent (the popup it's nested in), not a hardcoded `None` — the same
/// parent `dispatch_outcome_popups`/`dispatch_outcome_panels` already
/// compute via `surface_parent_for` for this module's own floating content.
#[test]
fn test_event_loop_tooltip_dispatch_carries_floating_parent_when_embedded() {
    let id = ModuleId::new(42);
    let parent_id = ModuleId::new(7);
    let mon = MonitorId::new("DP-1");

    let hub = Arc::new(SignalHub::new(Config::default()));
    let sm = Arc::new(MockSurfaceManager);
    let layout_sender = Arc::new(MockLayoutSender);
    let (display_tx, display_rx) = std::sync::mpsc::channel();
    let display_sender = Arc::new(ChannelDisplaySender::new(display_tx));
    let ui_sender = Arc::new(MockUiSender);
    let (_tx, rx) = tokio::sync::watch::channel(HashMap::new());
    // This module is itself embedded (via `ui.module()`) inside module
    // `parent_id`'s open popup — exactly the "module-in-popup" case.
    let ctx = ModuleContext::new(id, hub, sm, layout_sender, display_sender, ui_sender, rx)
        .with_parent(Some(parent_id))
        .with_surface(LayoutSurface::Popup);

    let mut event_loop = EventLoop::new(
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

    let outcome = crate::features::module_runtime::domain::RenderOutcome::new(
        None,
        vec![],
        make_test_rect_node(),
        None,
        FloatingLayouts::new(None, None, Some(make_test_rect_node())),
    );
    event_loop.dispatch_render_outcome(&mon, &outcome, Scale::new(1.0));

    let cmd = display_rx
        .try_recv()
        .expect("Should have sent ShowFloatingSurface for the tooltip");
    match cmd {
        DisplayCommand::ShowFloatingSurface(
            crate::features::layout_engine::domain::ShowFloating { kind, parent, .. },
        ) => {
            assert_eq!(kind, FloatingKind::Tooltip);
            assert_eq!(
                parent,
                Some(FloatingKind::Popup(PopupTarget::new(parent_id, mon)))
            );
        }
        _ => panic!("Expected ShowFloatingSurface, got {cmd:?}"),
    }
}
