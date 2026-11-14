use super::TaffyLayoutAdapter;
use super::calc_resolve::resolve_pending_calcs;
use super::diff::diff;
use super::leaf_size::size_to_dimension;
use super::reconciler::{apply_patch, build_layout_state};
use super::render_tree_builder::build_render_tree;
use super::tree_builder::TaffyTreeBuilder;
use crate::features::layout_engine::domain::{LayoutError, RenderNode, StyledNode, TextMeasurer};
use crate::shared::primitives::SizeConstraint;
use crate::shared::primitives::geometry::{Position, Size};
use crate::utils::f32_to_u32;
use taffy::style::Style;
use taffy::tree::NodeId;

/// A pinned axis becomes a definite available space; an unpinned axis is
/// `MaxContent` — "as much room as the content wants" — which is exactly
/// what an entirely unconstrained layout already used on both axes, so a
/// fully-`None` `SizeConstraint` reproduces today's intrinsic-everywhere
/// behaviour unchanged.
fn axis_available_space(pinned: Option<u32>) -> taffy::style::AvailableSpace {
    pinned.map_or(taffy::style::AvailableSpace::MaxContent, |px| {
        taffy::style::AvailableSpace::Definite(f32::from(u16::try_from(px).unwrap_or(u16::MAX)))
    })
}

impl TaffyLayoutAdapter {
    /// Pins the root's size to `constraint` and returns the style to
    /// restore afterwards. A pinned axis is the embedding module's host box,
    /// which the child fills (the Shadow DOM split): the root takes the
    /// pinned size itself, not merely the room to grow into, so `1fr`
    /// tracks and `space-between` have a definite box to distribute.
    fn pin_root(
        &mut self,
        root: NodeId,
        constraint: SizeConstraint,
    ) -> Result<Style, LayoutError> {
        let engine_err = |e: taffy::TaffyError| LayoutError::EngineError(e.to_string());
        let original = self.taffy.style(root).map_err(engine_err)?.clone();
        let mut pinned = original.clone();
        if let Some(w) = constraint.width() {
            pinned.size.width = size_to_dimension(w);
        }
        if let Some(h) = constraint.height() {
            pinned.size.height = size_to_dimension(h);
        }
        self.taffy.set_style(root, pinned).map_err(engine_err)?;
        Ok(original)
    }

    pub(super) fn layout_pass(
        &mut self,
        node: &StyledNode,
        measurer: &mut dyn TextMeasurer,
        start_pos: Position,
        constraint: SizeConstraint,
    ) -> Result<RenderNode, LayoutError> {
        let mut builder = TaffyTreeBuilder::new(&mut self.taffy);
        let new_state = if let Some(state) = &self.state {
            let patch = diff(state, node, measurer);
            apply_patch(&mut builder, patch, measurer)?
        } else {
            build_layout_state(&mut builder, node, measurer)?
        };

        let root_node_id = new_state.root_node;

        let available_space = taffy::geometry::Size {
            width: axis_available_space(constraint.width()),
            height: axis_available_space(constraint.height()),
        };

        // A pinned root is laid out twice: unpinned first, whose size is
        // what an embedder may measure it by, then pinned to the host box.
        self.intrinsic = None;
        let unpinned_root = if constraint.width().is_some() || constraint.height().is_some() {
            self.taffy
                .compute_layout(root_node_id, taffy::geometry::Size { width: axis_available_space(None), height: axis_available_space(None) })
                .map_err(|e| LayoutError::EngineError(e.to_string()))?;
            let size = self
                .taffy
                .layout(root_node_id)
                .map_err(|e| LayoutError::EngineError(e.to_string()))?
                .size;
            self.intrinsic = Some(Size::new(f32_to_u32(size.width), f32_to_u32(size.height)));
            Some(self.pin_root(root_node_id, constraint)?)
        } else {
            None
        };

        // Compute layout
        self.taffy
            .compute_layout(root_node_id, available_space)
            .map_err(|e| LayoutError::EngineError(e.to_string()))?;

        // A node styled with `calc(<percent>% + <px>px)` — e.g.
        // `width: calc(100% - 2rem)` — can't resolve its percent term
        // until its parent's size from this first pass is known. Patch any
        // such nodes with the now-concrete pixel value and lay out again;
        // ordinary trees (no calc()) never pay this cost.
        if resolve_pending_calcs(&mut self.taffy, &new_state) {
            self.taffy
                .compute_layout(root_node_id, available_space)
                .map_err(|e| LayoutError::EngineError(e.to_string()))?;
        }

        // Build RenderNode tree
        let render_tree = build_render_tree(&self.taffy, root_node_id, node, start_pos)?;

        // The diff only patches nodes whose style changed, so the root must
        // go back to its own style for the next pass to start clean.
        if let Some(original) = unpinned_root {
            self.taffy
                .set_style(root_node_id, original)
                .map_err(|e| LayoutError::EngineError(e.to_string()))?;
        }

        tracing::trace!(
            rect = ?render_tree.rect(),
            "Calculated layout render tree"
        );

        self.state = Some(new_state);

        Ok(render_tree)
    }
}
