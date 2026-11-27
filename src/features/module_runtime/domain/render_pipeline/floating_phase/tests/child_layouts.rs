use super::super::{
    FloatingCache, FloatingContentCaches, FloatingEngine, FloatingEngines, FloatingLayouts,
    FloatingPass, layout_floating,
};
use super::{Spec, content, panel_on_every_edge};
use crate::features::layout_engine::domain::LayoutRoot;
use crate::features::layout_engine::domain::{
    LayoutError, RenderNode, StyledNode, StyledPanel, StyledPopup, TextMeasurer,
};
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::module_runtime::test_support::MockCanvasFactory;
use crate::features::styling::domain::ComputedStyle;
use crate::features::vdom::domain::{
    AnchorDirection, InteractedNodes, InteractionContext, NodePath, NodeRef, PanelLayer,
    SurfaceSpace,
};
use crate::shared::primitives::geometry::{Position, Rect, Scale, Size};
use crate::shared::primitives::{
    ChildModuleLayout, LayoutSurface, ModuleKey, ModuleName, ModuleOptions, MonitorFocus, MonitorId,
};
use std::collections::HashMap;

/// Lays any content out as one embedded module named `name`, on the content's own surface.
struct ModuleEngine {
    name: &'static str,
}

impl LayoutEnginePort for ModuleEngine {
    fn calculate_layout(
        &mut self,
        root: LayoutRoot,
        _measurer: &mut dyn TextMeasurer,
    ) -> Result<RenderNode, LayoutError> {
        Ok(RenderNode::Module {
            path: NodePath::root_in(root.node().path().surface()),
            node_key: None,
            rect: Rect::new(Position::new(0, 0), Size::new(30, 12)),
            key: ModuleKey::from_name(ModuleName::new(self.name)),
            options: ModuleOptions::default(),
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        })
    }
}

/// A bar whose first child opens a popup and second a panel, carrying its own tooltip.
fn bar(popup: &StyledNode, panel: &StyledNode, tooltip: &StyledNode) -> RenderNode {
    let button = Spec {
        popup: Some(StyledPopup::new(
            Box::new(popup.clone()),
            AnchorDirection::Bottom,
            None,
        )),
        ..Spec::at_path(SurfaceSpace::Bar, &[0])
    }
    .leaf();
    let launcher = Spec {
        panel: Some(StyledPanel::new(
            Box::new(panel.clone()),
            panel_on_every_edge(PanelLayer::Top),
        )),
        ..Spec::at_path(SurfaceSpace::Bar, &[1])
    }
    .leaf();
    Spec {
        tooltip: Some(tooltip.clone()),
        rect: Rect::new(Position::new(0, 0), Size::new(90, 20)),
        ..Spec::at_path(SurfaceSpace::Bar, &[])
    }
    .flex(vec![button, launcher])
}

fn names(layouts: &[ChildModuleLayout]) -> Vec<(&str, LayoutSurface)> {
    layouts
        .iter()
        .map(|layout| (layout.key().name().as_str(), layout.surface()))
        .collect()
}

const MONITOR: &str = "HDMI-A-1";

/// The popup, panel and tooltip contents the tests open.
fn contents() -> [StyledNode; 3] {
    [
        content(SurfaceSpace::Popup, "calendar popup", None),
        content(SurfaceSpace::Panel, "launcher panel", None),
        content(SurfaceSpace::Tooltip, "clock tooltip", None),
    ]
}

/// The bar's root hovered on a focused monitor.
fn bar_hovered() -> InteractionContext {
    let hovered = NodeRef::new(Vec::new(), NodePath::root_in(SurfaceSpace::Bar));
    InteractionContext::new(
        InteractedNodes::new(Some(hovered), None, None),
        MonitorFocus::Focused,
    )
}

/// Lays out the floating content `bar` opens on `MONITOR`, with the bar hovered, by engines that
/// lay any content out as the module calendar (popup), launcher (panel) or weather (tooltip).
fn lay_out(bar: &RenderNode, caches: FloatingContentCaches<'_>) -> FloatingLayouts {
    let interaction = bar_hovered();
    let (mut calendar, mut launcher, mut weather) = (
        ModuleEngine { name: "calendar" },
        ModuleEngine { name: "launcher" },
        ModuleEngine { name: "weather" },
    );
    let FloatingContentCaches {
        popup,
        panel,
        tooltip,
    } = caches;
    let engines = FloatingEngines {
        popup: FloatingEngine::new(&mut calendar, popup),
        panel: FloatingEngine::new(&mut launcher, panel),
        tooltip: FloatingEngine::new(&mut weather, tooltip),
    };
    let mut pass = FloatingPass {
        monitor_id: &MonitorId::new(MONITOR),
        canvas_factory: &mut MockCanvasFactory,
        scale: Scale::new(1.0),
        current_child_sizes: None,
        interaction: Some(&interaction),
    };
    layout_floating(bar, engines, &mut pass)
}

/// The modules embedded in the popup, the panel and the tooltip are reported in that order.
#[test]
fn floating_modules_are_listed_per_kind() {
    let [popup, panel, tooltip] = contents();
    let (mut popup_cache, mut panel_cache, mut tooltip_cache) =
        (HashMap::new(), HashMap::new(), HashMap::new());
    let caches = FloatingContentCaches {
        popup: &mut popup_cache,
        panel: &mut panel_cache,
        tooltip: &mut tooltip_cache,
    };
    let layouts = lay_out(&bar(&popup, &panel, &tooltip), caches);
    assert_eq!(
        names(&layouts.module_layouts().collect::<Vec<_>>()),
        vec![
            ("calendar", LayoutSurface::Popup),
            ("launcher", LayoutSurface::Panel),
            ("weather", LayoutSurface::Tooltip),
        ]
    );
}

/// Each kind caches its own content under the monitor it was laid out for.
#[test]
fn each_floating_kind_caches_its_own_content() {
    let [popup, panel, tooltip] = contents();
    let (mut popup_cache, mut panel_cache, mut tooltip_cache) =
        (HashMap::new(), HashMap::new(), HashMap::new());
    let caches = FloatingContentCaches {
        popup: &mut popup_cache,
        panel: &mut panel_cache,
        tooltip: &mut tooltip_cache,
    };
    lay_out(&bar(&popup, &panel, &tooltip), caches);
    let monitor = MonitorId::new(MONITOR);
    let content_of = |cache: &FloatingCache| cache.get(&monitor).map(|entry| entry.0.clone());
    assert_eq!(content_of(&popup_cache), Some(popup));
    assert_eq!(content_of(&panel_cache), Some(panel));
    assert_eq!(content_of(&tooltip_cache), Some(tooltip));
}
