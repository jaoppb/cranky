mod content;
mod panel;
mod popup;
mod tooltip;

pub use panel::layout_panel;
pub use popup::{layout_popup, PopupRenderLayout};

use crate::features::layout_engine::domain::RenderNode;
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::vdom::domain::InteractionContext;
use crate::shared::primitives::geometry::Scale;
use crate::shared::primitives::{ChildModuleLayout, ChildSizesMap};
use crate::shared::rendering::ports::canvas::CanvasFactory;
use tooltip::layout_tooltip;

/// The layout results for whatever floating content is currently open on a monitor.
///
/// Computed once per render, from the same measurer, scale and font fallback as
/// the bar tree, so hit-testing and painting always agree.
#[derive(Debug, Clone, Default)]
pub struct FloatingLayouts {
    popup: Option<PopupRenderLayout>,
    panel: Option<RenderNode>,
    tooltip: Option<RenderNode>,
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
        }
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
pub fn layout_floating<F: CanvasFactory>(
    render_node: &RenderNode,
    interaction: Option<&InteractionContext>,
    popup_layout_engine: &mut dyn LayoutEnginePort,
    panel_layout_engine: &mut dyn LayoutEnginePort,
    tooltip_layout_engine: &mut dyn LayoutEnginePort,
    canvas_factory: &mut F,
    scale: Scale,
    current_child_sizes: Option<&ChildSizesMap>,
    child_layouts: &mut Vec<ChildModuleLayout>,
) -> FloatingLayouts {
    let popup = render_node.find_popup_with_anchor().and_then(|anchored| {
        let layout = layout_popup(
            &anchored,
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
        tooltip_layout_engine,
        canvas_factory,
        scale,
        current_child_sizes,
    )
    .inspect(|node| child_layouts.extend(node.collect_module_layouts()));

    FloatingLayouts {
        popup,
        panel,
        tooltip,
    }
}
