use crate::features::vdom::domain::{InteractionContext, NodeRef};
use crate::shared::primitives::MonitorId;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default)]
#[allow(clippy::struct_field_names)]
pub struct PointerHandler {
    pub(crate) hovered_nodes: HashMap<MonitorId, NodeRef>,
    pub(crate) active_nodes: HashMap<MonitorId, NodeRef>,
    pub(crate) focused_nodes: HashMap<MonitorId, NodeRef>,
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

    /// Drops hover/active/focus state for any monitor no longer in `live`,
    /// in one call rather than three parallel `.retain()`s — a removed
    /// monitor's stale interaction state must not resurrect if its id is
    /// reused by a later hotplug.
    pub fn retain_monitors(&mut self, live: &HashSet<MonitorId>) {
        self.hovered_nodes.retain(|id, _| live.contains(id));
        self.active_nodes.retain(|id, _| live.contains(id));
        self.focused_nodes.retain(|id, _| live.contains(id));
    }
}
