use super::TaffyLayoutAdapter;
use super::calc_resolve::resolve_pending_calcs;
use super::diff::diff;
use super::leaf_size::size_to_dimension;
use super::reconciler::{LayoutState, apply_patch, build_layout_state};
use super::render_tree_builder::{TaffyNode, build_render_tree};
use super::tree_builder::TaffyTreeBuilder;
use crate::features::layout_engine::domain::{
    LayoutError, LayoutRoot, RenderNode, StyledNode, TextMeasurer,
};
use crate::shared::primitives::SizeConstraint;
use crate::shared::primitives::geometry::Size;
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
    fn pin_root(&mut self, root: NodeId, constraint: SizeConstraint) -> Result<Style, LayoutError> {
        let engine_err = |error: taffy::TaffyError| LayoutError::EngineError(error.to_string());
        let original = self.taffy.style(root).map_err(engine_err)?.clone();
        let mut pinned = original.clone();
        if let Some(width) = constraint.width() {
            pinned.size.width = size_to_dimension(width);
        }
        if let Some(height) = constraint.height() {
            pinned.size.height = size_to_dimension(height);
        }
        self.taffy.set_style(root, pinned).map_err(engine_err)?;
        Ok(original)
    }

    pub(super) fn layout_pass(
        &mut self,
        root: &LayoutRoot,
        measurer: &mut dyn TextMeasurer,
    ) -> Result<RenderNode, LayoutError> {
        let node = root.node();
        let constraint = root.constraint();
        let new_state = self.reconcile(node, measurer)?;
        let root_node_id = new_state.root_node;
        let available_space = taffy::geometry::Size {
            width: axis_available_space(constraint.width()),
            height: axis_available_space(constraint.height()),
        };
        let unpinned_root = self.measure_and_pin(root_node_id, constraint)?;
        self.compute(root_node_id, available_space)?;
        // A node styled with `calc(<percent>% + <px>px)` — e.g.
        // `width: calc(100% - 2rem)` — can't resolve its percent term
        // until its parent's size from this first pass is known. Patch any
        // such nodes with the now-concrete pixel value and lay out again;
        // ordinary trees (no calc()) never pay this cost.
        if resolve_pending_calcs(&mut self.taffy, &new_state) {
            self.compute(root_node_id, available_space)?;
        }
        let render_tree = build_render_tree(
            TaffyNode::new(&self.taffy, root_node_id),
            node,
            root.origin(),
        )?;
        self.restore_root(root_node_id, unpinned_root)?;
        log_render_tree(&render_tree);
        self.state = Some(new_state);
        Ok(render_tree)
    }

    /// Brings the taffy tree in line with `node`: patches the last layout's tree with the
    /// difference, or builds a new one on the first pass.
    fn reconcile(
        &mut self,
        node: &StyledNode,
        measurer: &mut dyn TextMeasurer,
    ) -> Result<LayoutState, LayoutError> {
        let mut builder = TaffyTreeBuilder::new(&mut self.taffy);
        if let Some(state) = &self.state {
            let patch = diff(state, node, measurer);
            apply_patch(&mut builder, patch, measurer)
        } else {
            build_layout_state(&mut builder, node, measurer)
        }
    }

    /// A pinned root is laid out twice: unpinned first, whose size is what an embedder may
    /// measure it by and becomes the intrinsic size, then pinned to the host box. Returns the
    /// root's own style to restore after the pinned pass, or `None` when no axis is pinned.
    fn measure_and_pin(
        &mut self,
        root_node_id: NodeId,
        constraint: SizeConstraint,
    ) -> Result<Option<Style>, LayoutError> {
        self.intrinsic = None;
        if constraint.width().is_none() && constraint.height().is_none() {
            return Ok(None);
        }
        let unconstrained = taffy::geometry::Size {
            width: axis_available_space(None),
            height: axis_available_space(None),
        };
        self.compute(root_node_id, unconstrained)?;
        let size = self
            .taffy
            .layout(root_node_id)
            .map_err(|error| LayoutError::EngineError(error.to_string()))?
            .size;
        self.intrinsic = Some(Size::new(f32_to_u32(size.width), f32_to_u32(size.height)));
        Ok(Some(self.pin_root(root_node_id, constraint)?))
    }

    /// Lays the tree under `root_node_id` out in `available_space`.
    fn compute(
        &mut self,
        root_node_id: NodeId,
        available_space: taffy::geometry::Size<taffy::style::AvailableSpace>,
    ) -> Result<(), LayoutError> {
        self.taffy
            .compute_layout(root_node_id, available_space)
            .map_err(|error| LayoutError::EngineError(error.to_string()))
    }

    /// The diff only patches nodes whose style changed, so a pinned root goes back to its own
    /// style for the next pass to start clean.
    fn restore_root(
        &mut self,
        root_node_id: NodeId,
        unpinned_root: Option<Style>,
    ) -> Result<(), LayoutError> {
        if let Some(original) = unpinned_root {
            self.taffy
                .set_style(root_node_id, original)
                .map_err(|error| LayoutError::EngineError(error.to_string()))?;
        }
        Ok(())
    }
}

/// Traces the rect of the render tree a layout pass built.
fn log_render_tree(render_tree: &RenderNode) {
    tracing::trace!(
        rect = ?render_tree.rect(),
        "Calculated layout render tree"
    );
}
