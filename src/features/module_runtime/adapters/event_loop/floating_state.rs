use crate::features::layout_engine::domain::RenderNode;
use crate::shared::primitives::MonitorId;
use std::collections::{HashMap, HashSet};

/// Per-monitor popup/panel bookkeeping for one module's `EventLoop`, grouped
/// so dropping a monitor is a single `retain_monitors` call instead of four
/// parallel `.retain()`s that are easy to forget to keep in sync.
#[derive(Debug, Default)]
pub(super) struct FloatingState {
    active_popups: HashSet<MonitorId>,
    active_panels: HashSet<MonitorId>,
    popup_render_trees: HashMap<MonitorId, RenderNode>,
    panel_render_trees: HashMap<MonitorId, RenderNode>,
}

impl FloatingState {
    pub(super) fn retain_monitors(&mut self, live: &HashSet<MonitorId>) {
        self.active_popups.retain(|id| live.contains(id));
        self.active_panels.retain(|id| live.contains(id));
        self.popup_render_trees.retain(|id, _| live.contains(id));
        self.panel_render_trees.retain(|id, _| live.contains(id));
    }

    pub(super) const fn active_popups_mut(&mut self) -> &mut HashSet<MonitorId> {
        &mut self.active_popups
    }

    pub(super) const fn active_panels_mut(&mut self) -> &mut HashSet<MonitorId> {
        &mut self.active_panels
    }

    pub(super) const fn popup_render_trees(&self) -> &HashMap<MonitorId, RenderNode> {
        &self.popup_render_trees
    }

    pub(super) const fn panel_render_trees(&self) -> &HashMap<MonitorId, RenderNode> {
        &self.panel_render_trees
    }

    pub(super) const fn popup_render_trees_mut(&mut self) -> &mut HashMap<MonitorId, RenderNode> {
        &mut self.popup_render_trees
    }

    pub(super) const fn panel_render_trees_mut(&mut self) -> &mut HashMap<MonitorId, RenderNode> {
        &mut self.panel_render_trees
    }

    /// Both render-tree maps at once, so a caller needing to mutate both
    /// (e.g. `update_floating_trees`) doesn't have to take two overlapping
    /// `&mut self` borrows through separate accessor calls.
    pub(super) const fn render_trees_mut(
        &mut self,
    ) -> (
        &mut HashMap<MonitorId, RenderNode>,
        &mut HashMap<MonitorId, RenderNode>,
    ) {
        (&mut self.popup_render_trees, &mut self.panel_render_trees)
    }
}
