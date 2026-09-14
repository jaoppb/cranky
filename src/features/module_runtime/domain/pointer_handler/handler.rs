use crate::features::layout_engine::domain::StyledNode;
use crate::features::vdom::domain::{InteractionContext, NodeRef};
use crate::shared::primitives::geometry::Position;
use crate::shared::primitives::MonitorId;
use std::collections::HashMap;

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
}
