//! What dispatching one render outcome sends to the surface manager: the buffer, under the
//! module's embedding site.
use super::lifecycle::make_test_rect_node;
use crate::features::layout_engine::domain::{DisplayCommandSender, FloatingKind, PopupTarget};
use crate::features::module_runtime::adapters::event_loop::runner::EventLoop;
use crate::features::module_runtime::application::ModuleContext;
use crate::features::module_runtime::domain::RenderServices;
use crate::features::module_runtime::domain::render_pipeline::{EmbedReport, PipelineLayout};
use crate::features::module_runtime::domain::{FloatingLayouts, RenderOutcome};
use crate::features::module_runtime::ports::LayoutEventSender;
use crate::features::module_runtime::test_support::{
    MockCanvasFactory, MockDisplaySender, MockLayoutSender, MockUiSender, RecordingSurfaceManager,
    TestModulePort,
};
use crate::features::styling::adapters::fs_loader::CompositeStyleResolver;
use crate::features::vdom::adapters::DefaultVdomDiffAdapter;
use crate::features::vdom::domain::VNode;
use crate::shared::config::domain::Config;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::geometry::{Position, Scale, Size};
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::primitives::{LayoutSurface, ModuleId, MonitorId};
use crate::shared::wayland::ports::SurfaceParent;
use std::collections::HashMap;
use std::sync::Arc;

pub(super) fn event_loop<LS: LayoutEventSender, DS: DisplayCommandSender>(
    ctx: ModuleContext<LS, DS, MockUiSender>,
) -> EventLoop<MockCanvasFactory, LS, DS, MockUiSender> {
    EventLoop::new(
        Box::new(TestModulePort::new(VNode::new_rect(
            None, None, None, None, None,
        ))),
        ctx,
        RenderServices::new(
            MockCanvasFactory,
            Arc::new(CompositeStyleResolver::new(vec![])),
            Arc::new(DefaultVdomDiffAdapter::new()),
        ),
    )
}

#[test]
fn an_embedded_module_submits_its_buffer_under_its_site() {
    let monitor = MonitorId::new("HDMI-A-1");
    let surfaces = Arc::new(RecordingSurfaceManager::default());
    let (_tx, rx) = tokio::sync::watch::channel(HashMap::new());
    let ctx = ModuleContext::new(
        ModuleId::new(42),
        Arc::new(SignalHub::new(Config::default())),
        surfaces.clone(),
        Arc::new(MockLayoutSender),
        Arc::new(MockDisplaySender),
        Arc::new(MockUiSender),
        rx,
    )
    .with_parent(Some(ModuleId::new(7)))
    .with_surface(LayoutSurface::Panel);
    let buffer = RenderBuffer::new(vec![0u8; 6 * 4 * 4], Size::new(6, 4));

    event_loop(ctx).dispatch_render_outcome(
        &monitor,
        &RenderOutcome::new(
            PipelineLayout::new(
                make_test_rect_node(),
                FloatingLayouts::default(),
                EmbedReport::new(None, vec![]),
            ),
            Some((buffer.clone(), Position::new(5, 3))),
            Scale::new(1.0),
        ),
    );

    let frames = std::mem::take(&mut *surfaces.frames.lock().unwrap());
    let [(frame, parent)] = frames.as_slice() else {
        panic!("expected one frame, got {}", frames.len());
    };
    assert_eq!(frame.module_id(), ModuleId::new(42));
    assert_eq!(frame.monitor_id(), &monitor);
    assert_eq!(frame.position(), Position::new(5, 3));
    assert_eq!(frame.buffer(), &buffer);
    assert_eq!(
        parent,
        &Some(SurfaceParent::Floating(FloatingKind::Panel(
            PopupTarget::new(ModuleId::new(7), monitor)
        )))
    );
}

#[test]
fn a_top_level_module_submits_its_buffer_with_no_parent_and_nothing_without_one() {
    let surfaces = Arc::new(RecordingSurfaceManager::default());
    let (_tx, rx) = tokio::sync::watch::channel(HashMap::new());
    let ctx = ModuleContext::new(
        ModuleId::new(3),
        Arc::new(SignalHub::new(Config::default())),
        surfaces.clone(),
        Arc::new(MockLayoutSender),
        Arc::new(MockDisplaySender),
        Arc::new(MockUiSender),
        rx,
    );
    let mut event_loop = event_loop(ctx);
    let buffer = RenderBuffer::new(vec![0u8; 2 * 3 * 4], Size::new(2, 3));
    let monitor = MonitorId::new("DP-2");

    let empty = RenderOutcome::new(
        PipelineLayout::new(
            make_test_rect_node(),
            FloatingLayouts::default(),
            EmbedReport::new(None, vec![]),
        ),
        None,
        Scale::new(1.0),
    );
    event_loop.dispatch_render_outcome(&monitor, &empty);
    assert!(surfaces.frames.lock().unwrap().is_empty());

    let painted = RenderOutcome::new(
        PipelineLayout::new(
            make_test_rect_node(),
            FloatingLayouts::default(),
            EmbedReport::new(None, vec![]),
        ),
        Some((buffer, Position::new(0, 0))),
        Scale::new(1.0),
    );
    event_loop.dispatch_render_outcome(&monitor, &painted);
    let frames = std::mem::take(&mut *surfaces.frames.lock().unwrap());
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].0.module_id(), ModuleId::new(3));
    assert_eq!(frames[0].1, None);
}
