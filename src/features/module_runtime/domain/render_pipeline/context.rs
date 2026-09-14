use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::styling::ports::StyleResolverPort;
use crate::features::vdom::domain::{InteractionContext, VNode};
use crate::shared::primitives::geometry::{Rect, Scale};
use crate::shared::primitives::ChildSizesMap;
use crate::shared::rendering::ports::canvas::CanvasFactory;

pub struct LayoutContext<'a, F: CanvasFactory> {
    pub scale: Scale,
    pub style_resolver: &'a dyn StyleResolverPort,
    pub current_bounds: Option<Rect>,
    pub current_child_sizes: Option<&'a ChildSizesMap>,
    pub interaction_context: Option<InteractionContext>,
    pub canvas_factory: &'a mut F,
    pub layout_engine: &'a mut dyn LayoutEnginePort,
    /// Separate from `layout_engine`, and from each other, so laying out a
    /// popup, panel or tooltip's content never disturbs the bar tree's
    /// incremental layout state — or each other's, when more than one is
    /// open at once. Each engine instance tracks a single tree.
    pub popup_layout_engine: &'a mut dyn LayoutEnginePort,
    pub panel_layout_engine: &'a mut dyn LayoutEnginePort,
    pub tooltip_layout_engine: &'a mut dyn LayoutEnginePort,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PipelineDiff {
    new_vdom: VNode,
    vdom_dirty: bool,
    bounds_changed: bool,
    child_sizes_changed: bool,
}

impl PipelineDiff {
    #[must_use]
    pub const fn new(
        new_vdom: VNode,
        vdom_dirty: bool,
        bounds_changed: bool,
        child_sizes_changed: bool,
    ) -> Self {
        Self {
            new_vdom,
            vdom_dirty,
            bounds_changed,
            child_sizes_changed,
        }
    }

    #[must_use]
    pub const fn new_vdom(&self) -> &VNode {
        &self.new_vdom
    }

    #[must_use]
    pub const fn vdom_dirty(&self) -> bool {
        self.vdom_dirty
    }

    #[must_use]
    pub const fn bounds_changed(&self) -> bool {
        self.bounds_changed
    }

    #[must_use]
    pub const fn child_sizes_changed(&self) -> bool {
        self.child_sizes_changed
    }
}
