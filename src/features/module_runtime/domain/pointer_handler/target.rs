use crate::features::layout_engine::domain::RenderNode;
use crate::shared::primitives::MonitorId;

/// Where a pointer event landed: the monitor it happened on, which keys the pointer state, and
/// the laid-out tree of the surface it hit there, which the handlers hit-test.
#[derive(Debug, Clone, Copy)]
pub(in crate::features::module_runtime) struct PointerTarget<'a> {
    monitor_id: &'a MonitorId,
    tree: &'a RenderNode,
}

impl<'a> PointerTarget<'a> {
    #[must_use]
    pub(crate) const fn new(monitor_id: &'a MonitorId, tree: &'a RenderNode) -> Self {
        Self { monitor_id, tree }
    }

    #[must_use]
    pub(crate) const fn monitor_id(&self) -> &'a MonitorId {
        self.monitor_id
    }

    #[must_use]
    pub(crate) const fn tree(&self) -> &'a RenderNode {
        self.tree
    }
}
