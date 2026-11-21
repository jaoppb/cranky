//! What one pipeline run produces: the laid-out tree, the painted buffer, child layouts, floating
//! layouts and any size change.
use super::floating_phase::{FloatingHold, FloatingLayouts, PopupRenderLayout};
use crate::features::layout_engine::domain::RenderNode;
use crate::shared::primitives::ChildModuleLayout;
use crate::shared::primitives::geometry::{Position, Size};
use crate::shared::primitives::render::RenderBuffer;

/// Value Object representing a module size transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizeChange {
    old: Size,
    new: Size,
    settled_width: Option<u32>,
}

impl SizeChange {
    #[must_use]
    pub(crate) const fn new(old: Size, new: Size) -> Self {
        Self {
            old,
            new,
            settled_width: None,
        }
    }

    /// The width the module was laid out at once its embedder pinned one;
    /// changes even when the intrinsic size does not.
    #[must_use]
    pub(crate) const fn with_settled_width(mut self, settled_width: Option<u32>) -> Self {
        self.settled_width = settled_width;
        self
    }

    #[must_use]
    pub(crate) const fn settled_width(&self) -> Option<u32> {
        self.settled_width
    }

    #[must_use]
    pub(crate) const fn new_size(&self) -> Size {
        self.new
    }
}

/// Everything one render of a module on one monitor produced, for the event loop to dispatch: the
/// buffer, any size change, child module layouts and floating layouts.
#[derive(Debug, Clone)]
pub struct RenderOutcome {
    size_change: Option<SizeChange>,
    child_layouts: Vec<ChildModuleLayout>,
    render_tree: RenderNode,
    buffer: Option<(RenderBuffer, Position)>,
    floating: FloatingLayouts,
}

impl RenderOutcome {
    #[must_use]
    pub(crate) const fn new(
        size_change: Option<SizeChange>,
        child_layouts: Vec<ChildModuleLayout>,
        render_tree: RenderNode,
        buffer: Option<(RenderBuffer, Position)>,
        floating: FloatingLayouts,
    ) -> Self {
        Self {
            size_change,
            child_layouts,
            render_tree,
            buffer,
            floating,
        }
    }

    #[must_use]
    pub(crate) const fn popup_layout(&self) -> Option<&PopupRenderLayout> {
        self.floating.popup()
    }

    #[must_use]
    pub(crate) const fn panel_layout(&self) -> Option<&RenderNode> {
        self.floating.panel()
    }

    #[must_use]
    pub(crate) const fn tooltip_layout(&self) -> Option<&RenderNode> {
        self.floating.tooltip()
    }

    #[must_use]
    pub(crate) const fn popup_hold(&self) -> FloatingHold {
        self.floating.popup_hold()
    }

    #[must_use]
    pub(crate) const fn panel_hold(&self) -> FloatingHold {
        self.floating.panel_hold()
    }

    #[must_use]
    pub(crate) const fn tooltip_hold(&self) -> FloatingHold {
        self.floating.tooltip_hold()
    }

    #[must_use]
    pub(crate) const fn size_change(&self) -> Option<&SizeChange> {
        self.size_change.as_ref()
    }

    #[must_use]
    pub(crate) fn child_layouts(&self) -> &[ChildModuleLayout] {
        &self.child_layouts
    }

    /// The bar tree this render laid out, positioned from the module's own origin.
    #[must_use]
    pub const fn render_tree(&self) -> &RenderNode {
        &self.render_tree
    }

    #[must_use]
    pub(crate) const fn buffer(&self) -> Option<&(RenderBuffer, Position)> {
        self.buffer.as_ref()
    }
}
