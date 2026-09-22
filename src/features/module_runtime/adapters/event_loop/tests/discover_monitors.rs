use crate::features::module_runtime::adapters::event_loop::runner::EventLoop;
use crate::features::module_runtime::application::ModuleContext;
use crate::features::module_runtime::domain::{PointerHandler, RenderPipeline};
use crate::features::module_runtime::test_support::{
    MockCanvasFactory, MockDisplaySender, MockLayoutSender, MockSurfaceManager, MockUiSender,
    TestModulePort,
};
use crate::features::styling::adapters::fs_loader::CompositeStyleResolver;
use crate::features::vdom::adapters::DefaultVdomDiffAdapter;
use crate::features::vdom::domain::VNode;
use crate::shared::config::domain::Config;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::ModuleId;
use crate::shared::primitives::MonitorId;
use crate::shared::primitives::geometry::Scale;
use std::collections::HashMap;
use std::sync::Arc;

#[test]
fn test_discover_monitors_reads_wayland_monitor_scales() {
    let id = ModuleId::new(1);
    let hub = Arc::new(SignalHub::new(Config::default()));

    // Wayland is the sole discovery source - a monitor only exists once
    // its wl_output has published a scale.
    let mut scales = hub.monitor_scales_rx().borrow().clone();
    scales.insert(MonitorId::new("HDMI-A-1"), Scale::new(1.0));
    let _ = hub.monitor_scales_tx().send(scales);

    let sm = Arc::new(MockSurfaceManager);
    let layout_sender = Arc::new(MockLayoutSender);
    let display_sender = Arc::new(MockDisplaySender);
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

    let discovered = event_loop.discover_monitors();
    assert!(discovered.contains(&MonitorId::new("HDMI-A-1")));
}

#[test]
fn test_discover_monitors_ignores_module_sizes_cache() {
    // module_sizes is a cache that only ever gains entries - it must not be
    // treated as a discovery source, or a disconnected monitor a module
    // once reported a size for would stay "discovered" forever.
    let id = ModuleId::new(1);
    let hub = Arc::new(SignalHub::new(Config::default()));

    let mut sizes = hub.module_sizes_rx().borrow().clone();
    sizes.insert(
        MonitorId::new("GHOST-1"),
        crate::shared::primitives::ChildSizesMap::new(),
    );
    let _ = hub.module_sizes_tx().send(sizes);

    let sm = Arc::new(MockSurfaceManager);
    let layout_sender = Arc::new(MockLayoutSender);
    let display_sender = Arc::new(MockDisplaySender);
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

    let discovered = event_loop.discover_monitors();
    assert!(!discovered.contains(&MonitorId::new("GHOST-1")));
}
