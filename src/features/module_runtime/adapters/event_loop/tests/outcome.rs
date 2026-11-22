//! What dispatching one render outcome sends to the surface manager: the buffer, under the
//! module's embedding site.
use super::fixture::{ChannelEventLoop, PARENT, Wiring, embed, empty_hub, wired_event_loop};
use super::lifecycle::make_test_rect_node;
use crate::features::layout_engine::domain::{FloatingKind, PopupTarget};
use crate::features::module_runtime::domain::render_pipeline::{EmbedReport, PipelineLayout};
use crate::features::module_runtime::domain::{FloatingLayouts, RenderOutcome};
use crate::features::module_runtime::test_support::TestModulePort;
use crate::features::vdom::domain::{ElementSelectors, Interactions, VNode};
use crate::shared::primitives::geometry::{Position, Scale, Size};
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::primitives::{LayoutSurface, ModuleId, MonitorId};
use crate::shared::wayland::ports::SurfaceParent;

/// An event loop for module 42 rendering a plain rect, on a hub with no monitor.
pub(super) fn plain_event_loop() -> (ChannelEventLoop, Wiring) {
    let rect = VNode::new_rect(ElementSelectors::default(), Interactions::default());
    wired_event_loop(TestModulePort::new(rect), empty_hub())
}

/// An outcome at scale 1 with nothing open, painted into `buffer` at `position` if any.
fn painted(buffer: Option<(RenderBuffer, Position)>) -> RenderOutcome {
    RenderOutcome::new(
        PipelineLayout::new(
            make_test_rect_node(),
            FloatingLayouts::default(),
            EmbedReport::new(None, vec![]),
        ),
        buffer,
        Scale::new(1.0),
    )
}

#[test]
fn an_embedded_module_submits_its_buffer_under_its_site() {
    let monitor = MonitorId::new("HDMI-A-1");
    let (event_loop, wiring) = plain_event_loop();
    let mut event_loop = embed(event_loop, LayoutSurface::Panel);
    let buffer = RenderBuffer::new(vec![0u8; 6 * 4 * 4], Size::new(6, 4));

    event_loop.dispatch_render_outcome(
        &monitor,
        &painted(Some((buffer.clone(), Position::new(5, 3)))),
    );

    let frames = std::mem::take(&mut *wiring.surfaces.frames.lock().unwrap());
    let [(frame, parent)] = frames.as_slice() else {
        panic!("expected one frame, got {}", frames.len());
    };
    assert_eq!(frame.module_id(), ModuleId::new(42));
    assert_eq!(frame.monitor_id(), &monitor);
    assert_eq!(frame.position(), Position::new(5, 3));
    assert_eq!(frame.buffer(), &buffer);
    let site = FloatingKind::Panel(PopupTarget::new(PARENT, monitor));
    assert_eq!(parent, &Some(SurfaceParent::Floating(site)));
}

#[test]
fn a_top_level_module_submits_its_buffer_with_no_parent_and_nothing_without_one() {
    let (mut event_loop, wiring) = plain_event_loop();
    let buffer = RenderBuffer::new(vec![0u8; 2 * 3 * 4], Size::new(2, 3));
    let monitor = MonitorId::new("DP-2");

    event_loop.dispatch_render_outcome(&monitor, &painted(None));
    assert!(wiring.surfaces.frames.lock().unwrap().is_empty());

    event_loop.dispatch_render_outcome(&monitor, &painted(Some((buffer, Position::new(0, 0)))));
    let frames = std::mem::take(&mut *wiring.surfaces.frames.lock().unwrap());
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].0.module_id(), ModuleId::new(42));
    assert_eq!(frames[0].1, None);
}
