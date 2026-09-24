use crate::features::layout_engine::domain::StyledNode;
use crate::features::vdom::domain::{InteractionContext, NodeRef};
use crate::shared::primitives::MonitorId;
use crate::shared::primitives::geometry::Position;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default)]
pub struct PointerHandler {
    pub(crate) hovered_nodes: HashMap<MonitorId, NodeRef>,
    pub(crate) active_nodes: HashMap<MonitorId, NodeRef>,
    pub(crate) focused_nodes: HashMap<MonitorId, NodeRef>,
    pub(crate) last_tooltip: Option<StyledNode>,
    pub(crate) last_pointer_pos: Option<(MonitorId, Position)>,
}

impl PointerHandler {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn hovered_node(&self, monitor_id: &MonitorId) -> Option<&NodeRef> {
        self.hovered_nodes.get(monitor_id)
    }

    #[must_use]
    pub fn active_node(&self, monitor_id: &MonitorId) -> Option<&NodeRef> {
        self.active_nodes.get(monitor_id)
    }

    #[must_use]
    pub fn focused_node(&self, monitor_id: &MonitorId) -> Option<&NodeRef> {
        self.focused_nodes.get(monitor_id)
    }

    #[must_use]
    pub fn interaction_context(
        &self,
        monitor_id: &MonitorId,
        is_monitor_focused: bool,
    ) -> InteractionContext {
        InteractionContext::new(
            self.hovered_nodes.get(monitor_id).cloned(),
            self.active_nodes.get(monitor_id).cloned(),
            self.focused_nodes.get(monitor_id).cloned(),
            is_monitor_focused,
        )
    }

    #[must_use]
    pub const fn last_tooltip(&self) -> Option<&StyledNode> {
        self.last_tooltip.as_ref()
    }

    #[must_use]
    pub const fn last_pointer_pos(&self) -> Option<&(MonitorId, Position)> {
        self.last_pointer_pos.as_ref()
    }

    /// Drops hover/active/focus state for any monitor no longer in `live`,
    /// in one call rather than three parallel `.retain()`s. Also drops the
    /// last known pointer position (and its associated tooltip cache) if it
    /// was on a monitor that's now gone - otherwise a later reconnect of
    /// the same monitor id would resurrect a stale position/tooltip with
    /// no new `PointerMotion` event to justify it.
    pub fn retain_monitors(&mut self, live: &HashSet<MonitorId>) {
        self.hovered_nodes.retain(|id, _| live.contains(id));
        self.active_nodes.retain(|id, _| live.contains(id));
        self.focused_nodes.retain(|id, _| live.contains(id));
        if let Some((monitor_id, _)) = &self.last_pointer_pos
            && !live.contains(monitor_id)
        {
            self.last_pointer_pos = None;
            self.last_tooltip = None;
        }
    }
}
