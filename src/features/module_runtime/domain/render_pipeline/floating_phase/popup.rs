use super::content::layout_floating_content;
use crate::features::layout_engine::domain::popup::AnchoredPopup;
use crate::features::layout_engine::domain::RenderNode;
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::shared::primitives::geometry::{Rect, Scale};
use crate::shared::primitives::{ChildSizesMap, PopupOffset};
use crate::shared::rendering::ports::canvas::CanvasFactory;

/// The pipeline's layout of an open popup: its anchor (on the parent's own
/// tree), the laid-out content, and the offset the script requested.
#[derive(Debug, Clone, PartialEq)]
pub struct PopupRenderLayout {
    anchor_rect: Rect,
    node: RenderNode,
    offset: Option<PopupOffset>,
}

impl PopupRenderLayout {
    #[must_use]
    pub const fn new(anchor_rect: Rect, node: RenderNode, offset: Option<PopupOffset>) -> Self {
        Self {
            anchor_rect,
            node,
            offset,
        }
    }

    #[must_use]
    pub const fn anchor_rect(&self) -> Rect {
        self.anchor_rect
    }

    #[must_use]
    pub const fn node(&self) -> &RenderNode {
        &self.node
    }

    #[must_use]
    pub const fn offset(&self) -> Option<PopupOffset> {
        self.offset
    }
}

/// Lays out an anchored popup's content with the pipeline's own measurer,
/// scale and font fallback — the single layout pass that both hit-testing
/// and painting now read from.
pub fn layout_popup<F: CanvasFactory>(
    anchored: &AnchoredPopup<'_>,
    layout_engine: &mut dyn LayoutEnginePort,
    canvas_factory: &mut F,
    scale: Scale,
    current_child_sizes: Option<&ChildSizesMap>,
) -> Option<PopupRenderLayout> {
    let node = layout_floating_content(
        layout_engine,
        canvas_factory,
        scale,
        current_child_sizes,
        anchored.layout(),
    )?;
    Some(PopupRenderLayout {
        anchor_rect: *anchored.anchor_rect(),
        node,
        offset: anchored.popup().offset(),
    })
}
