//! The inputs of one pipeline run: the layout context the event loop hands in, the view of it the
//! diff phase compares, and the diff result the diff phase passes to the layout phase.
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::module_runtime::domain::Change;
use crate::features::styling::ports::StyleResolverPort;
use crate::features::vdom::domain::{InteractionContext, VNode};
use crate::features::vdom::ports::VdomDiffPort;
use crate::shared::primitives::geometry::{Rect, Scale};
use crate::shared::primitives::{ChildSizesMap, ModuleKey, MonitorId, SizeConstraint};
use crate::shared::rendering::ports::canvas::CanvasFactory;
use std::collections::HashMap;

/// Everything one module's layout and paint phases need on one monitor.
///
/// The monitor and its scale, the style resolver, the space it may take, child sizes, pointer
/// state and the layout engines it lays out with.
pub struct LayoutContext<'a, F: CanvasFactory, E: LayoutEnginePort> {
    /// The monitor this render is for, which keys every piece of state the pipeline keeps.
    pub monitor_id: &'a MonitorId,
    /// The monitor's output scale, used to measure text and to size the painted buffer.
    pub scale: Scale,
    /// Resolves the module's stylesheets against each node of its tree.
    pub style_resolver: &'a dyn StyleResolverPort,
    /// Terminal lazy-spawn failures for this module's own children, keyed by
    /// the failed child's `ModuleKey` — pre-filtered from the hub's
    /// site-keyed map, since this module only ever needs to know about its
    /// own embedding sites. Consulted right before style resolution to swap
    /// a failed `Module` node for an error placeholder.
    pub child_errors: &'a HashMap<ModuleKey, String>,
    /// Still used for the module's own painted position — where its buffer
    /// goes on whatever surface it lives on. No longer consulted for sizing:
    /// see `current_constraint`.
    pub current_bounds: Option<Rect>,
    /// What this module's own layout may use as available space, per axis —
    /// for the root, always both axes pinned to its layer-surface bounds;
    /// for an embedded module, only the axes its parent's CSS pinned. Never
    /// derived from `current_bounds`, so a parent resizing a child's slot
    /// can't feed back into a bigger slot next frame — see decision 10.
    pub current_constraint: SizeConstraint,
    /// Sizes embedded child modules last reported on this monitor, used to size their host boxes.
    pub current_child_sizes: Option<&'a ChildSizesMap>,
    /// The hovered, pressed and focused nodes on this monitor, which drive pseudo-class styles.
    pub interaction_context: Option<InteractionContext>,
    /// Makes the text measurer layout uses and the canvas the paint phase draws into.
    pub canvas_factory: &'a mut F,
    /// Incremental layout engine for the module's own bar tree.
    pub layout_engine: &'a mut E,
    /// Separate from `layout_engine`, and from each other, so laying out a
    /// popup, panel or tooltip's content never disturbs the bar tree's
    /// incremental layout state — or each other's, when more than one is
    /// open at once. Each engine instance tracks a single tree.
    pub popup_layout_engine: &'a mut E,
    /// Layout engine reserved for the module's panel content.
    pub panel_layout_engine: &'a mut E,
    /// Layout engine reserved for the module's tooltip content.
    pub tooltip_layout_engine: &'a mut E,
}

impl<F: CanvasFactory, E: LayoutEnginePort> std::fmt::Debug for LayoutContext<'_, F, E> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LayoutContext")
            .field("monitor_id", &self.monitor_id)
            .field("scale", &self.scale)
            .field("current_bounds", &self.current_bounds)
            .field("current_constraint", &self.current_constraint)
            .field("child_errors", &self.child_errors)
            .field("current_child_sizes", &self.current_child_sizes)
            .field("interaction_context", &self.interaction_context)
            .finish_non_exhaustive()
    }
}

impl<F: CanvasFactory, E: LayoutEnginePort> LayoutContext<'_, F, E> {
    /// The monitor, bounds, child sizes and pointer state of this render, for the diff phase to
    /// compare with `vdom_diff` against the last render on that monitor.
    #[must_use]
    pub(crate) fn diff_input<'b>(&'b self, vdom_diff: &'b dyn VdomDiffPort) -> DiffInput<'b> {
        DiffInput {
            monitor_id: self.monitor_id,
            vdom_diff,
            bounds: self.current_bounds,
            child_sizes: self.current_child_sizes,
            interaction: self.interaction_context.as_ref(),
        }
    }
}

/// What the diff phase compares with the last render on one monitor.
///
/// The bounds, child sizes and pointer state a render is made with, which are compared with those
/// of the last render on that monitor, and the differ that compares the script's new tree with the
/// last one.
#[derive(Clone, Copy)]
pub struct DiffInput<'a> {
    /// The monitor whose last render the inputs are compared with.
    pub monitor_id: &'a MonitorId,
    /// Compares the script's new tree with the one it rendered last on this monitor.
    pub vdom_diff: &'a dyn VdomDiffPort,
    /// Where the module is placed on this monitor now, if anywhere.
    pub bounds: Option<Rect>,
    /// Sizes its embedded child modules reported on this monitor.
    pub child_sizes: Option<&'a ChildSizesMap>,
    /// The hovered, pressed and focused nodes on this monitor.
    pub interaction: Option<&'a InteractionContext>,
}

impl std::fmt::Debug for DiffInput<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DiffInput")
            .field("monitor_id", &self.monitor_id)
            .field("bounds", &self.bounds)
            .field("child_sizes", &self.child_sizes)
            .field("interaction", &self.interaction)
            .finish_non_exhaustive()
    }
}

/// What the diff phase found on one monitor: the script's new tree and which inputs changed, which
/// decides whether the layout phase lays it out again or reuses the last tree.
#[derive(Debug, Clone, PartialEq)]
pub struct PipelineDiff {
    pub(super) new_vdom: VNode,
    pub(super) vdom: Change,
    pub(super) bounds: Change,
    pub(super) child_sizes: Change,
}

impl PipelineDiff {
    #[must_use]
    pub(crate) const fn new_vdom(&self) -> &VNode {
        &self.new_vdom
    }

    #[must_use]
    pub(crate) const fn vdom_dirty(&self) -> bool {
        self.vdom.is_changed()
    }

    #[must_use]
    pub(crate) const fn bounds_changed(&self) -> bool {
        self.bounds.is_changed()
    }

    #[must_use]
    pub(crate) const fn child_sizes_changed(&self) -> bool {
        self.child_sizes.is_changed()
    }
}
