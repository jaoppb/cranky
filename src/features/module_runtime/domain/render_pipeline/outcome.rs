//! What one pipeline run produces: the laid-out tree, the painted buffer, child layouts, floating
//! layouts and any size change.
use super::floating_phase::{FloatingHold, FloatingLayouts, PopupRenderLayout};
use crate::features::layout_engine::domain::RenderNode;
use crate::shared::primitives::ChildModuleLayout;
use crate::shared::primitives::geometry::{Position, Scale, Size};
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

/// What one layout tells the modules around it: its own size change, for its embedder, and the
/// layouts of the modules it embeds, for each of them.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EmbedReport {
    size_change: Option<SizeChange>,
    child_layouts: Vec<ChildModuleLayout>,
}

impl EmbedReport {
    #[must_use]
    pub(crate) const fn new(
        size_change: Option<SizeChange>,
        child_layouts: Vec<ChildModuleLayout>,
    ) -> Self {
        Self {
            size_change,
            child_layouts,
        }
    }

    #[must_use]
    pub(crate) const fn size_change(&self) -> Option<&SizeChange> {
        self.size_change.as_ref()
    }

    #[must_use]
    pub(crate) fn child_layouts(&self) -> &[ChildModuleLayout] {
        &self.child_layouts
    }
}

/// What the layout phase produced on one monitor: the bar tree, the layouts of whatever floating
/// content is open, and what to report to the modules around it.
#[derive(Debug, Clone)]
pub struct PipelineLayout {
    render_tree: RenderNode,
    floating: FloatingLayouts,
    report: EmbedReport,
}

impl PipelineLayout {
    #[must_use]
    pub(crate) const fn new(
        render_tree: RenderNode,
        floating: FloatingLayouts,
        report: EmbedReport,
    ) -> Self {
        Self {
            render_tree,
            floating,
            report,
        }
    }

    #[must_use]
    pub(crate) const fn render_tree(&self) -> &RenderNode {
        &self.render_tree
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn floating(&self) -> &FloatingLayouts {
        &self.floating
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn report(&self) -> &EmbedReport {
        &self.report
    }
}

/// Everything one render of a module on one monitor produced, for the event loop to dispatch: the
/// layout, the buffer, and the scale both were made at, which floating content is painted at too.
#[derive(Debug, Clone)]
pub struct RenderOutcome {
    layout: PipelineLayout,
    buffer: Option<(RenderBuffer, Position)>,
    scale: Scale,
}

impl RenderOutcome {
    #[must_use]
    pub(crate) const fn new(
        layout: PipelineLayout,
        buffer: Option<(RenderBuffer, Position)>,
        scale: Scale,
    ) -> Self {
        Self {
            layout,
            buffer,
            scale,
        }
    }

    #[must_use]
    pub(crate) const fn popup_layout(&self) -> Option<&PopupRenderLayout> {
        self.layout.floating.popup()
    }

    #[must_use]
    pub(crate) const fn panel_layout(&self) -> Option<&RenderNode> {
        self.layout.floating.panel()
    }

    #[must_use]
    pub(crate) const fn tooltip_layout(&self) -> Option<&RenderNode> {
        self.layout.floating.tooltip()
    }

    #[must_use]
    pub(crate) const fn popup_hold(&self) -> FloatingHold {
        self.layout.floating.popup_hold()
    }

    #[must_use]
    pub(crate) const fn panel_hold(&self) -> FloatingHold {
        self.layout.floating.panel_hold()
    }

    #[must_use]
    pub(crate) const fn tooltip_hold(&self) -> FloatingHold {
        self.layout.floating.tooltip_hold()
    }

    #[must_use]
    pub(crate) const fn size_change(&self) -> Option<&SizeChange> {
        self.layout.report.size_change()
    }

    #[must_use]
    pub(crate) fn child_layouts(&self) -> &[ChildModuleLayout] {
        self.layout.report.child_layouts()
    }

    /// The bar tree this render laid out, positioned from the module's own origin.
    #[must_use]
    pub const fn render_tree(&self) -> &RenderNode {
        &self.layout.render_tree
    }

    #[must_use]
    pub(crate) const fn buffer(&self) -> Option<&(RenderBuffer, Position)> {
        self.buffer.as_ref()
    }

    #[must_use]
    pub(crate) const fn scale(&self) -> Scale {
        self.scale
    }
}
