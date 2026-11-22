use super::super::{FloatingEngine, FloatingEngines, FloatingPass, layout_floating};
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

/// The modules embedded in the popup, the panel and the tooltip are reported in that order, and
/// each kind caches its own content under the monitor it was laid out for.
#[test]
fn floating_modules_are_listed_per_kind_and_each_kind_caches_its_own_content() {
    let popup = content(SurfaceSpace::Popup, "calendar popup", None);
    let panel = content(SurfaceSpace::Panel, "launcher panel", None);
    let tooltip = content(SurfaceSpace::Tooltip, "clock tooltip", None);
    let hovered = NodeRef::new(Vec::new(), NodePath::root_in(SurfaceSpace::Bar));
    let interaction = InteractionContext::new(
        InteractedNodes::new(Some(hovered), None, None),
        MonitorFocus::Focused,
    );
    let monitor = MonitorId::new("HDMI-A-1");
    let (mut popup_cache, mut panel_cache, mut tooltip_cache) =
        (HashMap::new(), HashMap::new(), HashMap::new());

    let layouts = layout_floating(
        &bar(&popup, &panel, &tooltip),
        FloatingEngines {
            popup: FloatingEngine {
                engine: &mut ModuleEngine { name: "calendar" },
                cache: &mut popup_cache,
            },
            panel: FloatingEngine {
                engine: &mut ModuleEngine { name: "launcher" },
                cache: &mut panel_cache,
            },
            tooltip: FloatingEngine {
                engine: &mut ModuleEngine { name: "weather" },
                cache: &mut tooltip_cache,
            },
        },
        &mut FloatingPass {
            monitor_id: &monitor,
            canvas_factory: &mut MockCanvasFactory,
            scale: Scale::new(1.0),
            current_child_sizes: None,
            interaction: Some(&interaction),
        },
    );

    assert_eq!(
        names(&layouts.module_layouts().collect::<Vec<_>>()),
        vec![
            ("calendar", LayoutSurface::Popup),
            ("launcher", LayoutSurface::Panel),
            ("weather", LayoutSurface::Tooltip),
        ]
    );
    assert_eq!(
        popup_cache.get(&monitor).map(|entry| &entry.0),
        Some(&popup)
    );
    assert_eq!(
        panel_cache.get(&monitor).map(|entry| &entry.0),
        Some(&panel)
    );
    assert_eq!(
        tooltip_cache.get(&monitor).map(|entry| &entry.0),
        Some(&tooltip)
    );
}
