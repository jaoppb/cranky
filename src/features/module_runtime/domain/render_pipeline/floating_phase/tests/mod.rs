mod child_layouts;
mod layout_floating;
mod tooltip;

use crate::features::layout_engine::domain::LayoutRoot;
use crate::features::layout_engine::domain::{
    LayoutError, RenderNode, StyledNode, StyledPanel, StyledPopup, TextMeasurer,
};
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::styling::domain::ComputedStyle;
use crate::features::vdom::domain::{
    NodeKey, NodePath, PanelAnchor, PanelLayer, PanelSpec, PanelSurface, SurfaceSpace, TextContent,
    VNode,
};
use crate::shared::primitives::geometry::{Position, Rect, Size};

/// Records the content it was asked to lay out, and returns a node on the
/// same surface that keeps the content's own tooltip.
#[derive(Default)]
struct RecordingEngine {
    laid_out: Vec<StyledNode>,
}

impl LayoutEnginePort for RecordingEngine {
    fn calculate_layout(
        &mut self,
        root: LayoutRoot,
        _measurer: &mut dyn TextMeasurer,
    ) -> Result<RenderNode, LayoutError> {
        let node = root.node().clone();
        let laid_out = Spec {
            tooltip: node.tooltip().cloned(),
            ..Spec::at_path(node.path().surface(), &[])
        }
        .leaf();
        self.laid_out.push(node);
        Ok(laid_out)
    }
}

/// Styled text content rooted in `surface`, as a floating kind's content is.
fn content(surface: SurfaceSpace, text: &str, tooltip: Option<StyledNode>) -> StyledNode {
    StyledNode::Text {
        path: NodePath::root_in(surface),
        node_key: None,
        text: TextContent::new(text.to_string()),
        style: ComputedStyle::default(),
        on_click: None,
        on_hover: None,
        tooltip: tooltip.map(Box::new),
        popup: None,
        panel: None,
    }
}

fn tooltip_content(text: &str) -> StyledNode {
    content(SurfaceSpace::Tooltip, text, None)
}

/// The surface of a panel on `layer` anchored to every edge, the rest left at its defaults.
fn panel_on_every_edge(layer: PanelLayer) -> PanelSurface {
    PanelSpec::new(Box::new(VNode::new_rect(None, None, None, None, None)))
        .with_layer(layer)
        .with_anchor(PanelAnchor::all())
        .surface()
}

/// Everything a test may want to set on a laid-out node; the rest is fixed.
struct Spec {
    path: NodePath,
    key: Option<&'static str>,
    rect: Rect,
    tooltip: Option<StyledNode>,
    popup: Option<StyledPopup>,
    panel: Option<StyledPanel>,
}

impl Spec {
    fn at_path(surface: SurfaceSpace, indices: &[usize]) -> Self {
        Self {
            path: NodePath::new(surface, indices.to_vec()),
            key: None,
            rect: Rect::new(Position::new(0, 0), Size::new(1, 1)),
            tooltip: None,
            popup: None,
            panel: None,
        }
    }

    fn leaf(self) -> RenderNode {
        RenderNode::Rect {
            path: self.path,
            node_key: self.key.map(|name| NodeKey::new(name).unwrap()),
            rect: self.rect,
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: self.tooltip.map(Box::new),
            popup: self.popup,
            panel: self.panel,
        }
    }

    fn flex(self, children: Vec<RenderNode>) -> RenderNode {
        RenderNode::Flex {
            path: self.path,
            node_key: self.key.map(|name| NodeKey::new(name).unwrap()),
            rect: self.rect,
            children,
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: self.tooltip.map(Box::new),
            popup: self.popup,
            panel: self.panel,
        }
    }
}
