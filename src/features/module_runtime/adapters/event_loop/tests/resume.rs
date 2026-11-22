use super::fixture::{ChannelEventLoop, Wiring, embed, hub_with_dp1, wired_event_loop};
use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::module_runtime::test_support::TestModulePort;
use crate::features::vdom::domain::{ElementSelectors, Interactions, VNode};
use crate::shared::primitives::geometry::{Position, Rect, Size};
use crate::shared::primitives::{ChildBounds, LayoutSurface, MonitorId, SizeConstraint};
use std::collections::HashMap;

type Layouts = HashMap<MonitorId, ChildBounds>;

/// An actor for module 42 embedded in module 7's popup — the calendar-in-clock-popup case — on a
/// hub where Wayland reports DP-1.
fn embedded_event_loop() -> (ChannelEventLoop, Wiring) {
    let rect = VNode::new_rect(ElementSelectors::default(), Interactions::default());
    let (event_loop, wiring) = wired_event_loop(TestModulePort::new(rect), hub_with_dp1(1.0));
    (embed(event_loop, LayoutSurface::Popup), wiring)
}

/// How many frames the actor handed to Wayland: the only observable proof that an embedded actor
/// actually re-painted.
fn submits(wiring: &Wiring) -> usize {
    wiring.surfaces.frames.lock().unwrap().len()
}

fn site(mon: &MonitorId) -> Layouts {
    let rect = Rect::new(Position::new(4, 6), Size::new(40, 30));
    let mut layouts = Layouts::new();
    layouts.insert(mon.clone(), ChildBounds::new(rect, SizeConstraint::none()));
    layouts
}

/// The popup embedding this actor closes (its rect is withdrawn) and then
/// reopens with the very same rect. The popup's surface — and the child's
/// pixels composited into it — were torn down on close, so the reopened
/// slot is a hole unless this actor re-submits its buffer, even though its
/// VDOM, rect, child sizes and interaction state are all unchanged.
#[test]
fn suspended_embedded_actor_rerenders_when_site_returns() {
    let mon = MonitorId::new("DP-1");
    let (mut event_loop, wiring) = embedded_event_loop();
    let layout_tx = &wiring.layouts;
    let mut engines: HashMap<MonitorId, TaffyLayoutAdapter> = HashMap::new();

    let _ = layout_tx.send(site(&mon));
    event_loop.render_all_monitors(&mut engines);
    assert_eq!(submits(&wiring), 1);

    // Popup closes: the parent withdraws the rect. Several passes happen
    // while suspended (any subscribed signal wakes the actor).
    let _ = layout_tx.send(Layouts::new());
    event_loop.render_all_monitors(&mut engines);
    event_loop.render_all_monitors(&mut engines);
    event_loop.render_all_monitors(&mut engines);
    assert_eq!(submits(&wiring), 1);

    // Popup reopens at the same rect. Relies on `retain_monitors` having
    // cleared the suspended monitor's rendered bounds: a pruning change that
    // keeps them would make this an unchanged no-op and leave a permanent
    // hole in the composited popup.
    let _ = layout_tx.send(site(&mon));
    event_loop.render_all_monitors(&mut engines);
    assert_eq!(submits(&wiring), 2);
}
