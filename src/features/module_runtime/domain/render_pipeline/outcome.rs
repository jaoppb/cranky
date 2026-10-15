use super::floating_phase::{FloatingLayouts, PopupRenderLayout};
use crate::features::layout_engine::domain::RenderNode;
use crate::shared::primitives::geometry::{Position, Size};
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::primitives::ChildModuleLayout;

/// Value Object representing a module size transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizeChange {
    old: Size,
    new: Size,
}

impl SizeChange {
    #[must_use]
    pub const fn new(old: Size, new: Size) -> Self {
        Self { old, new }
    }

    #[must_use]
    pub const fn old(&self) -> Size {
        self.old
    }

    #[must_use]
    pub const fn new_size(&self) -> Size {
        self.new
    }
}

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
    pub const fn new(
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
    pub const fn popup_layout(&self) -> Option<&PopupRenderLayout> {
        self.floating.popup()
    }

    #[must_use]
    pub const fn panel_layout(&self) -> Option<&RenderNode> {
        self.floating.panel()
    }

    #[must_use]
    pub const fn tooltip_layout(&self) -> Option<&RenderNode> {
        self.floating.tooltip()
    }

    #[must_use]
    pub const fn size_change(&self) -> Option<&SizeChange> {
        self.size_change.as_ref()
    }

    #[must_use]
    pub fn child_layouts(&self) -> &[ChildModuleLayout] {
        &self.child_layouts
    }

    #[must_use]
    pub const fn render_tree(&self) -> &RenderNode {
        &self.render_tree
    }

    #[must_use]
    pub const fn buffer(&self) -> Option<&(RenderBuffer, Position)> {
        self.buffer.as_ref()
    }

    #[must_use]
    pub fn into_buffer(self) -> Option<(RenderBuffer, Position)> {
        self.buffer
    }
}
