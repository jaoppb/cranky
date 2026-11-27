//! One render pass over every monitor: the state it drops for monitors that are gone, and what it
//! reads from the hub and the parent for each monitor it renders.
use super::fixture::{ChannelEventLoop, hub_with_dp1, wired_event_loop};
use super::lifecycle::make_test_rect_node;
use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::module_runtime::ports::LayoutEvent;
use crate::features::module_runtime::test_support::{RecordingModulePort, capture_logs};
use crate::features::vdom::domain::{
    ElementSelectors, Interactions, ModuleParams, NodePath, NodeRef, SurfaceSpace, TextContent,
    VNode,
};
use crate::shared::primitives::geometry::{Position, Rect, Size};
use crate::shared::primitives::{
    ChildBounds, ModuleId, ModuleKey, ModuleName, ModuleOptions, ModuleSite, MonitorId,
    SizeConstraint,
};
use std::collections::HashMap;
use std::sync::mpsc::Receiver;

type Engines = HashMap<MonitorId, TaffyLayoutAdapter>;

fn text() -> VNode {
    VNode::new_text(
        TextContent::new("bar".to_string()),
        ElementSelectors::default(),
        Interactions::default(),
    )
}

/// Gives `monitor` an open popup, panel and tooltip, child layouts and a hovered node.
fn fill_per_monitor_state(event_loop: &mut ChannelEventLoop, monitor: &MonitorId) {
    for trees in [
        &mut event_loop.popup_render_trees,
        &mut event_loop.panel_render_trees,
        &mut event_loop.tooltip_render_trees,
    ] {
        trees.insert(monitor.clone(), (None, make_test_rect_node()));
    }
    event_loop
        .child_layouts_present
        .insert(monitor.clone(), true);
    let hovered = NodeRef::new(Vec::new(), NodePath::root_in(SurfaceSpace::Bar));
    event_loop
        .pointer_handler
        .hovered
        .insert(monitor.clone(), hovered);
}

/// A pass drops the layout engine, floating trees, child-layout flag and pointer state of a
/// monitor Wayland no longer reports, and keeps those of the monitors it renders.
#[test]
fn a_monitor_that_is_gone_loses_every_per_monitor_state() {
    let (gone, dp1) = (MonitorId::new("GONE-1"), MonitorId::new("DP-1"));
    let (mut event_loop, _wiring) =
        wired_event_loop(RecordingModulePort::new(text(), vec![]), hub_with_dp1(1.0));
    let mut engines = Engines::from([(gone.clone(), TaffyLayoutAdapter::new())]);
    fill_per_monitor_state(&mut event_loop, &gone);
    fill_per_monitor_state(&mut event_loop, &dp1);

    event_loop.render_all_monitors(&mut engines);

    assert_eq!(engines.keys().collect::<Vec<_>>(), [&dp1]);
    for trees in [
        &event_loop.popup_render_trees,
        &event_loop.panel_render_trees,
        &event_loop.tooltip_render_trees,
    ] {
        assert!(!trees.contains_key(&gone));
    }
    assert_eq!(
        event_loop.child_layouts_present.keys().collect::<Vec<_>>(),
        [&dp1]
    );
    let hovered_monitors: Vec<_> = event_loop.pointer_handler.hovered.keys().collect();
    assert_eq!(hovered_monitors, [&dp1]);
}

fn module(name: &str) -> VNode {
    VNode::new_module(
        ModuleParams::new(ModuleName::new(name), None, ModuleOptions::default()),
        ElementSelectors::default(),
        Interactions::default(),
    )
}

/// The hub's entry for a failed child `name` embedded by module `parent`.
fn failed_child(parent: u32, name: &str) -> (ModuleSite, String) {
    let key = ModuleKey::from_name(ModuleName::new(name));
    let site = ModuleSite::new(Some(ModuleId::new(parent)), key);
    (site, "failed".to_string())
}

/// The names of the child layouts in the first event sent, which must report them.
fn reported_children(layout_events: &Receiver<LayoutEvent>) -> Vec<String> {
    let reported: Vec<_> = layout_events.try_iter().collect();
    let Some(LayoutEvent::ContainerLayoutsCalculated { layouts, .. }) = reported.first() else {
        panic!("expected the child layouts first, got {reported:?}");
    };
    let names = layouts.iter().map(|layout| layout.key().name().as_str());
    names.map(ToString::to_string).collect()
}

/// Only failures of this module's own children turn into placeholders; a failure the hub keeps
/// for another parent's child of the same name does not.
#[test]
fn only_this_modules_failed_children_become_placeholders() {
    let bar = VNode::new_flex(
        vec![module("calendar"), module("clock")],
        ElementSelectors::default(),
        Interactions::default(),
    );
    let (mut event_loop, wiring) =
        wired_event_loop(RecordingModulePort::new(bar, vec![]), hub_with_dp1(1.0));
    let errors = HashMap::from([failed_child(42, "calendar"), failed_child(7, "clock")]);
    wiring.hub.module_errors_tx().send_replace(errors);

    event_loop.render_all_monitors(&mut Engines::new());

    assert_eq!(reported_children(&wiring.sent.layout_events), ["clock"]);
}

/// The monitor's scale sizes the buffer, the parent's bounds place it, and the width its
/// constraint pins is the width the module reports it settled at.
#[test]
fn the_scale_and_the_parents_bounds_shape_what_is_painted_and_reported() {
    let (mut event_loop, wiring) =
        wired_event_loop(RecordingModulePort::new(text(), vec![]), hub_with_dp1(2.0));
    let rect = Rect::new(Position::new(4, 6), Size::new(10, 10));
    let bounds = ChildBounds::new(rect, SizeConstraint::new(Some(280), None));
    let layouts = HashMap::from([(MonitorId::new("DP-1"), bounds)]);
    wiring.layouts.send_replace(layouts);

    event_loop.render_all_monitors(&mut Engines::new());

    let frames = std::mem::take(&mut *wiring.surfaces.frames.lock().unwrap());
    let [(frame, None)] = frames.as_slice() else {
        panic!("expected one top-level frame, got {}", frames.len());
    };
    assert_eq!(frame.position(), Position::new(4, 6));
    assert_eq!((frame.buffer().width(), frame.buffer().height()), (20, 20));
    let reported: Vec<_> = wiring.sent.layout_events.try_iter().collect();
    assert!(
        matches!(
            reported.as_slice(),
            [LayoutEvent::ModuleSizeChanged {
                settled_width: Some(280),
                ..
            }]
        ),
        "expected one size change settled at 280, got {reported:?}"
    );
}

/// Each pass logs how long it took, under the renderer's target.
#[test]
fn each_pass_is_logged_with_its_duration() {
    let (mut event_loop, _wiring) =
        wired_event_loop(RecordingModulePort::new(text(), vec![]), hub_with_dp1(1.0));
    let logs = capture_logs(|| event_loop.render_all_monitors(&mut Engines::new()));
    let line = logs
        .lines()
        .find(|line| line.contains("Module UI updated"))
        .expect("the pass is logged");
    assert!(line.starts_with(
        "DEBUG cranky::features::module_runtime::adapters::event_loop::renderer: Module UI \
         updated module=42 duration_ms="
    ));
    assert!(line.contains(" duration_micros="));
}
