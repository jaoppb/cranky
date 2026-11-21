use super::content::{FloatingCache, FloatingPass, layout_floating_content_cached};
use crate::features::layout_engine::domain::RenderNode;
use crate::features::layout_engine::domain::popup::AnchoredPopup;
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::shared::primitives::PopupOffset;
use crate::shared::primitives::geometry::Rect;
use crate::shared::rendering::ports::canvas::CanvasFactory;

/// The pipeline's layout of an open popup: its anchor (on the parent's own
/// tree), the laid-out content, and the offset the script requested.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PopupRenderLayout {
    anchor_rect: Rect,
    node: RenderNode,
    offset: Option<PopupOffset>,
}

impl PopupRenderLayout {
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn new(
        anchor_rect: Rect,
        node: RenderNode,
        offset: Option<PopupOffset>,
    ) -> Self {
        Self {
            anchor_rect,
            node,
            offset,
        }
    }

    #[must_use]
    pub(crate) const fn anchor_rect(&self) -> Rect {
        self.anchor_rect
    }

    #[must_use]
    pub(crate) const fn node(&self) -> &RenderNode {
        &self.node
    }

    #[must_use]
    pub(crate) const fn offset(&self) -> Option<PopupOffset> {
        self.offset
    }
}

/// Lays out an anchored popup's content with the pipeline's own measurer,
/// scale and font fallback — the single layout pass that both hit-testing
/// and painting now read from.
pub(super) fn layout_popup<F: CanvasFactory, E: LayoutEnginePort>(
    anchored: &AnchoredPopup<'_>,
    cache: &mut FloatingCache,
    layout_engine: &mut E,
    pass: &mut FloatingPass<'_, F>,
) -> Option<PopupRenderLayout> {
    let node = layout_floating_content_cached(cache, layout_engine, pass, anchored.layout())?;
    Some(PopupRenderLayout {
        anchor_rect: *anchored.anchor_rect(),
        node,
        offset: anchored.popup().offset(),
    })
}
