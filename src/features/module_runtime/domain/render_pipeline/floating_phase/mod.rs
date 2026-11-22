//! Lays out the floating content (popup, panel, tooltip) a module's tree currently has open,
//! after the bar tree's own layout.
pub mod content;
mod gate;
mod panel;
pub mod popup;
#[cfg(test)]
mod tests;
mod tooltip;

pub(crate) use content::{FloatingCache, FloatingEngine, FloatingPass};
pub(super) use gate::apply_gate;
use panel::layout_panel;
pub(crate) use popup::PopupRenderLayout;
use popup::layout_popup;

use crate::features::layout_engine::domain::RenderNode;
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::shared::primitives::ChildModuleLayout;
use crate::shared::rendering::ports::canvas::CanvasFactory;
use tooltip::{HoverTrees, layout_tooltip};

/// The three floating kinds' per-monitor content caches.
///
/// Borrowed together so `layout_floating` can hand each kind its own cache
/// without borrowing `RenderPipeline` three separate times — see
/// `RenderPipeline::floating_content_caches_mut`.
pub(crate) struct FloatingContentCaches<'a> {
    pub(crate) popup: &'a mut FloatingCache,
    pub(crate) panel: &'a mut FloatingCache,
    pub(crate) tooltip: &'a mut FloatingCache,
}

/// One layout engine, with its content cache, per floating kind.
///
/// Sharing one would diff one kind's tree against another's cached layout
/// state whenever more than one is open at once.
pub(crate) struct FloatingEngines<'a, E: LayoutEnginePort> {
    pub(crate) popup: FloatingEngine<'a, E>,
    pub(crate) panel: FloatingEngine<'a, E>,
    pub(crate) tooltip: FloatingEngine<'a, E>,
}

/// Whether a floating surface must keep what it shows because an embedded child has not settled
/// on its host width yet, or may present its latest layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum FloatingHold {
    /// Leave the surface as it is: closed, or showing its previous frame.
    Held,
    /// Show, update or hide the surface from the latest layout.
    #[default]
    Present,
}

/// The layout results for whatever floating content is currently open on a monitor.
///
/// Computed once per render, from the same measurer, scale and font fallback as
/// the bar tree, so hit-testing and painting always agree.
#[derive(Debug, Clone, Default)]
pub struct FloatingLayouts {
    popup: Option<PopupRenderLayout>,
    panel: Option<RenderNode>,
    tooltip: Option<RenderNode>,
    /// Per kind: an embedded child has not settled on the host width yet,
    /// so the dispatcher must not present or update that surface.
    held: [FloatingHold; 3],
}

impl FloatingLayouts {
    #[must_use]
    pub(crate) const fn new(
        popup: Option<PopupRenderLayout>,
        panel: Option<RenderNode>,
        tooltip: Option<RenderNode>,
    ) -> Self {
        Self {
            popup,
            panel,
            tooltip,
            held: [FloatingHold::Present; 3],
        }
    }

    /// These layouts with the holds of the popup, the panel and the tooltip, in that order.
    #[must_use]
    pub(super) const fn with_held(mut self, held: [FloatingHold; 3]) -> Self {
        self.held = held;
        self
    }

    #[must_use]
    pub(crate) const fn popup_hold(&self) -> FloatingHold {
        self.held[0]
    }

    #[must_use]
    pub(crate) const fn panel_hold(&self) -> FloatingHold {
        self.held[1]
    }

    #[must_use]
    pub(crate) const fn tooltip_hold(&self) -> FloatingHold {
        self.held[2]
    }

    #[must_use]
    pub(crate) const fn popup(&self) -> Option<&PopupRenderLayout> {
        self.popup.as_ref()
    }

    #[must_use]
    pub(crate) const fn panel(&self) -> Option<&RenderNode> {
        self.panel.as_ref()
    }

    #[must_use]
    pub(crate) const fn tooltip(&self) -> Option<&RenderNode> {
        self.tooltip.as_ref()
    }

    /// The modules embedded in the popup, the panel and the tooltip, in that order.
    pub(crate) fn module_layouts(&self) -> impl Iterator<Item = ChildModuleLayout> + '_ {
        let popup = self.popup.iter().map(PopupRenderLayout::node);
        popup
            .chain(self.panel.iter())
            .chain(self.tooltip.iter())
            .flat_map(RenderNode::collect_module_layouts)
    }
}

/// Computes the floating layouts for whatever popup/panel/tooltip is
/// currently active; `FloatingLayouts::module_layouts` lists the module nodes
/// found inside them.
pub(crate) fn layout_floating<F: CanvasFactory, E: LayoutEnginePort>(
    render_node: &RenderNode,
    engines: FloatingEngines<'_, E>,
    pass: &mut FloatingPass<'_, F>,
) -> FloatingLayouts {
    let FloatingEngines {
        popup: popup_engine,
        panel: panel_engine,
        tooltip: tooltip_engine,
    } = engines;
    let popup = render_node
        .find_popup_with_anchor()
        .and_then(|anchored| layout_popup(&anchored, popup_engine, pass));
    let panel = render_node
        .find_panel()
        .and_then(|active| layout_panel(&active, panel_engine, pass));
    let trees = HoverTrees {
        bar: render_node,
        popup: popup.as_ref().map(PopupRenderLayout::node),
        panel: panel.as_ref(),
    };
    let tooltip = layout_tooltip(trees, tooltip_engine, pass);

    FloatingLayouts::new(popup, panel, tooltip)
}
