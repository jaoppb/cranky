mod content;
mod gate;
mod panel;
mod popup;
mod tooltip;

pub(super) use gate::apply_gate;
use panel::layout_panel;
use popup::layout_popup;
pub use content::FloatingCache;
pub use popup::PopupRenderLayout;

use crate::features::layout_engine::domain::RenderNode;
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::vdom::domain::InteractionContext;
use crate::shared::primitives::geometry::Scale;
use crate::shared::primitives::{ChildModuleLayout, ChildSizesMap, MonitorId};
use crate::shared::rendering::ports::canvas::CanvasFactory;
use tooltip::layout_tooltip;

/// The three floating kinds' per-monitor content caches.
///
/// Borrowed together so `layout_floating` can hand each kind its own cache
/// without borrowing `RenderPipeline` three separate times — see
/// `RenderPipeline::floating_content_caches_mut`.
pub struct FloatingContentCaches<'a> {
    pub popup: &'a mut FloatingCache,
    pub panel: &'a mut FloatingCache,
    pub tooltip: &'a mut FloatingCache,
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
    held: [bool; 3],
}

impl FloatingLayouts {
    #[must_use]
    pub const fn new(
        popup: Option<PopupRenderLayout>,
        panel: Option<RenderNode>,
        tooltip: Option<RenderNode>,
    ) -> Self {
        Self {
            popup,
            panel,
            tooltip,
            held: [false; 3],
        }
    }

    #[must_use]
    pub(super) const fn with_held(mut self, popup: bool, panel: bool, tooltip: bool) -> Self {
        self.held = [popup, panel, tooltip];
        self
    }

    #[must_use]
    pub const fn popup_held(&self) -> bool {
        self.held[0]
    }

    #[must_use]
    pub const fn panel_held(&self) -> bool {
        self.held[1]
    }

    #[must_use]
    pub const fn tooltip_held(&self) -> bool {
        self.held[2]
    }

    #[must_use]
    pub const fn popup(&self) -> Option<&PopupRenderLayout> {
        self.popup.as_ref()
    }

    #[must_use]
    pub const fn panel(&self) -> Option<&RenderNode> {
        self.panel.as_ref()
    }

    #[must_use]
    pub const fn tooltip(&self) -> Option<&RenderNode> {
        self.tooltip.as_ref()
    }
}

/// Computes the floating layouts for whatever popup/panel/tooltip is
/// currently active, extending `child_layouts` with any module nodes found
/// inside them.
///
/// Each floating kind gets its own layout engine — sharing one would diff
/// one kind's tree against another's cached layout state whenever more than
/// one is open at once.
#[allow(clippy::too_many_arguments)]
pub fn layout_floating<F: CanvasFactory, E: LayoutEnginePort>(
    render_node: &RenderNode,
    interaction: Option<&InteractionContext>,
    monitor_id: &MonitorId,
    caches: FloatingContentCaches<'_>,
    popup_layout_engine: &mut E,
    panel_layout_engine: &mut E,
    tooltip_layout_engine: &mut E,
    canvas_factory: &mut F,
    scale: Scale,
    current_child_sizes: Option<&ChildSizesMap>,
    child_layouts: &mut Vec<ChildModuleLayout>,
) -> FloatingLayouts {
    let FloatingContentCaches {
        popup: popup_cache,
        panel: panel_cache,
        tooltip: tooltip_cache,
    } = caches;
    let popup = render_node.find_popup_with_anchor().and_then(|anchored| {
        let layout = layout_popup(
            &anchored,
            popup_cache,
            monitor_id,
            popup_layout_engine,
            canvas_factory,
            scale,
            current_child_sizes,
        )?;
        child_layouts.extend(layout.node().collect_module_layouts());
        Some(layout)
    });
    let panel = render_node.find_panel().and_then(|active| {
        let node = layout_panel(
            &active,
            panel_cache,
            monitor_id,
            panel_layout_engine,
            canvas_factory,
            scale,
            current_child_sizes,
        )?;
        child_layouts.extend(node.collect_module_layouts());
        Some(node)
    });
    let tooltip = layout_tooltip(
        render_node,
        popup.as_ref().map(PopupRenderLayout::node),
        panel.as_ref(),
        interaction,
        tooltip_cache,
        monitor_id,
        tooltip_layout_engine,
        canvas_factory,
        scale,
        current_child_sizes,
    )
    .inspect(|node| child_layouts.extend(node.collect_module_layouts()));

    FloatingLayouts::new(popup, panel, tooltip)
}
