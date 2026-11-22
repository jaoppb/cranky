use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::module_runtime::adapters::event_loop::runner::EventLoop;
use crate::features::module_runtime::application::ModuleContext;
use crate::features::module_runtime::domain::RenderServices;
use crate::features::module_runtime::ports::{ModuleHandles, ModuleSenders};
use crate::features::module_runtime::test_support::{
    MockCanvasFactory, MockDisplaySender, MockLayoutSender, MockSurfaceManager, MockUiSender,
    TestModulePort,
};
use crate::features::styling::domain::{ComputedStyle, ElementQuery, InheritedStyle, PseudoClass};
use crate::features::styling::ports::StyleResolverPort;
use crate::features::vdom::adapters::DefaultVdomDiffAdapter;
use crate::features::vdom::domain::{ElementSelectors, Interactions, VNode};
use crate::features::workspaces::domain::MonitorName;
use crate::shared::config::domain::Config;
use crate::shared::events::signals::{HyprlandState, SignalHub};
use crate::shared::primitives::geometry::Scale;
use crate::shared::primitives::{ModuleId, MonitorId};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Counts the elements resolved with `:focus`.
#[derive(Default)]
struct FocusCounter {
    focused: AtomicUsize,
}

impl StyleResolverPort for FocusCounter {
    fn resolve_style(
        &self,
        query: &ElementQuery<'_>,
        _inherited: &InheritedStyle,
    ) -> (ComputedStyle, InheritedStyle) {
        if query.pseudo_classes().contains(&PseudoClass::Focused) {
            self.focused.fetch_add(1, Ordering::SeqCst);
        }
        (ComputedStyle::default(), InheritedStyle::default())
    }
}

/// Renders a top-level module once on DP-1 while Hyprland focuses
/// `focused`, and returns how many elements were resolved with `:focus`.
fn focused_elements(focused: &str) -> usize {
    let mon = MonitorId::new("DP-1");
    let hub = Arc::new(SignalHub::new(Config::default()));
    let mut scales = hub.monitor_scales_rx().borrow().clone();
    scales.insert(mon, Scale::new(1.0));
    let _ = hub.monitor_scales_tx().send(scales);
    hub.hyprland_tx().send_replace(HyprlandState::new(
        BTreeMap::new(),
        BTreeMap::new(),
        Some(MonitorName::new(focused)),
    ));

    let resolver = Arc::new(FocusCounter::default());
    let (_tx, rx) = tokio::sync::watch::channel(HashMap::new());
    let ctx = ModuleContext::new(
        ModuleId::new(42),
        ModuleHandles::new(
            hub,
            Arc::new(MockSurfaceManager),
            ModuleSenders::new(
                Arc::new(MockLayoutSender),
                Arc::new(MockDisplaySender),
                Arc::new(MockUiSender),
            ),
        ),
        rx,
    );
    let mut event_loop = EventLoop::new(
        Box::new(TestModulePort::new(VNode::new_rect(
            ElementSelectors::default(),
            Interactions::default(),
        ))),
        ctx,
        RenderServices::new(
            MockCanvasFactory,
            resolver.clone(),
            Arc::new(DefaultVdomDiffAdapter::new()),
        ),
    );
    let mut engines: HashMap<MonitorId, TaffyLayoutAdapter> = HashMap::new();
    event_loop.render_all_monitors(&mut engines);
    resolver.focused.load(Ordering::SeqCst)
}

/// The renderer gives the bar root `:focus` on the monitor Hyprland
/// focuses, and not on any other.
#[test]
fn test_render_focuses_the_bar_root_only_on_the_focused_monitor() {
    assert_eq!(focused_elements("DP-1"), 1);
    assert_eq!(focused_elements("HDMI-A-1"), 0);
}
