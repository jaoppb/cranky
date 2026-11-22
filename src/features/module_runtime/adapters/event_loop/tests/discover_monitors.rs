use super::fixture::{empty_hub, wired_event_loop};
use crate::features::module_runtime::adapters::event_loop::dispatch::{
    Embedding, discover_monitors,
};
use crate::features::module_runtime::test_support::TestModulePort;
use crate::features::vdom::domain::{ElementSelectors, Interactions, VNode};
use crate::shared::config::domain::Config;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::geometry::Scale;
use crate::shared::primitives::{ChildBounds, ChildSizesMap, MonitorId};
use std::collections::HashMap;
use std::sync::Arc;

/// What a top-level event loop on `hub` discovers with no layouts.
fn discovered_on(hub: Arc<SignalHub>) -> Vec<MonitorId> {
    let rect = VNode::new_rect(ElementSelectors::default(), Interactions::default());
    let (event_loop, _wiring) = wired_event_loop(TestModulePort::new(rect), hub);
    event_loop.discover_monitors(&HashMap::<MonitorId, ChildBounds>::new())
}

#[test]
fn test_discover_monitors_reads_wayland_monitor_scales() {
    let hub = empty_hub();

    // Wayland is the sole discovery source - a monitor only exists once
    // its wl_output has published a scale.
    let mut scales = hub.monitor_scales_rx().borrow().clone();
    scales.insert(MonitorId::new("HDMI-A-1"), Scale::new(1.0));
    let _ = hub.monitor_scales_tx().send(scales);

    assert!(discovered_on(hub).contains(&MonitorId::new("HDMI-A-1")));
}

#[test]
fn test_discover_monitors_ignores_module_sizes_cache() {
    // module_sizes is a cache that only ever gains entries - it must not be
    // treated as a discovery source, or a disconnected monitor a module
    // once reported a size for would stay "discovered" forever.
    let hub = empty_hub();

    let mut sizes = hub.module_sizes_rx().borrow().clone();
    sizes.insert(MonitorId::new("GHOST-1"), ChildSizesMap::new());
    let _ = hub.module_sizes_tx().send(sizes);

    assert!(!discovered_on(hub).contains(&MonitorId::new("GHOST-1")));
}

/// A monitor Wayland still reports, but whose embedding rect the parent
/// withdrew (absent from `layouts`) and that this actor has already
/// rendered for at least once (in `seen`), must stay suspended rather than
/// being rediscovered from `monitor_scales` alone — the two discovery axes
/// (does the monitor exist at all vs. does this actor still have a live
/// site on it) are independent, and only the first is Wayland's to answer.
#[test]
fn test_discover_monitors_suspends_seen_monitor_with_no_live_layout() {
    let hub = SignalHub::new(Config::default());
    let mut scales = hub.monitor_scales_rx().borrow().clone();
    scales.insert(MonitorId::new("DP-1"), Scale::new(1.0));
    let _ = hub.monitor_scales_tx().send(scales);

    let layouts: HashMap<MonitorId, crate::shared::primitives::ChildBounds> = HashMap::new();
    let mut seen = std::collections::HashSet::new();
    seen.insert(MonitorId::new("DP-1"));

    let discovered = discover_monitors(&hub, &layouts, Embedding::Embedded { seen: &seen });
    assert!(!discovered.contains(&MonitorId::new("DP-1")));
}

/// The same monitor, never rendered before (not in `seen`), still needs the
/// Wayland bootstrap so it can render once, unconstrained, before anyone
/// has assigned it a layout rect.
#[test]
fn test_discover_monitors_bootstraps_unseen_monitor_with_no_live_layout() {
    let hub = SignalHub::new(Config::default());
    let mut scales = hub.monitor_scales_rx().borrow().clone();
    scales.insert(MonitorId::new("DP-1"), Scale::new(1.0));
    let _ = hub.monitor_scales_tx().send(scales);

    let layouts: HashMap<MonitorId, crate::shared::primitives::ChildBounds> = HashMap::new();
    let seen = std::collections::HashSet::new();

    let discovered = discover_monitors(&hub, &layouts, Embedding::Embedded { seen: &seen });
    assert!(discovered.contains(&MonitorId::new("DP-1")));
}

/// A top-level actor has no embedding site to lose, so no rendered monitors
/// to consult: it covers every monitor Wayland reports, even one no layout
/// names.
#[test]
fn test_discover_monitors_top_level_covers_every_wayland_monitor() {
    let hub = SignalHub::new(Config::default());
    let mut scales = hub.monitor_scales_rx().borrow().clone();
    scales.insert(MonitorId::new("DP-1"), Scale::new(1.0));
    scales.insert(MonitorId::new("HDMI-A-1"), Scale::new(1.0));
    let _ = hub.monitor_scales_tx().send(scales);

    let layouts: HashMap<MonitorId, crate::shared::primitives::ChildBounds> = HashMap::new();

    let mut discovered = discover_monitors(&hub, &layouts, Embedding::TopLevel);
    discovered.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    assert_eq!(
        discovered,
        vec![MonitorId::new("DP-1"), MonitorId::new("HDMI-A-1")]
    );
}

/// An embedded actor keeps a seen monitor while its parent still gives it a
/// layout there, and ignores a layout for a monitor Wayland does not know.
#[test]
fn test_discover_monitors_embedded_keeps_seen_monitor_with_a_live_layout() {
    let hub = SignalHub::new(Config::default());
    let mut scales = hub.monitor_scales_rx().borrow().clone();
    scales.insert(MonitorId::new("DP-1"), Scale::new(1.0));
    let _ = hub.monitor_scales_tx().send(scales);

    let mut layouts: HashMap<MonitorId, u8> = HashMap::new();
    layouts.insert(MonitorId::new("DP-1"), 0);
    layouts.insert(MonitorId::new("GHOST-1"), 0);
    let mut seen = std::collections::HashSet::new();
    seen.insert(MonitorId::new("DP-1"));
    seen.insert(MonitorId::new("GHOST-1"));

    let discovered = discover_monitors(&hub, &layouts, Embedding::Embedded { seen: &seen });
    assert_eq!(discovered, vec![MonitorId::new("DP-1")]);
}
