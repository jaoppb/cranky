use crate::shared::primitives::{ModuleId, MonitorId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PopupTarget {
    module_id: ModuleId,
    monitor_id: MonitorId,
}

impl PopupTarget {
    #[must_use]
    pub const fn new(module_id: ModuleId, monitor_id: MonitorId) -> Self {
        Self {
            module_id,
            monitor_id,
        }
    }

    #[must_use]
    pub const fn module_id(&self) -> ModuleId {
        self.module_id
    }

    #[must_use]
    pub const fn monitor_id(&self) -> &MonitorId {
        &self.monitor_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FloatingKind {
    Tooltip,
    Popup(PopupTarget),
    Panel(PopupTarget),
}

pub struct PopupExclusivity;

impl PopupExclusivity {
    #[must_use]
    pub fn compute_conflicts(
        active_popups: &[PopupTarget],
        target: &PopupTarget,
        behavior: crate::shared::config::domain::PopupBehavior,
    ) -> Vec<PopupTarget> {
        match behavior {
            crate::shared::config::domain::PopupBehavior::PerModuleAndMonitor => active_popups
                .iter()
                .filter(|p| {
                    *p != target
                        && p.module_id() == target.module_id()
                        && p.monitor_id() == target.monitor_id()
                })
                .cloned()
                .collect(),
            crate::shared::config::domain::PopupBehavior::PerModule => active_popups
                .iter()
                .filter(|p| *p != target && p.module_id() == target.module_id())
                .cloned()
                .collect(),
            crate::shared::config::domain::PopupBehavior::PerMonitor => active_popups
                .iter()
                .filter(|p| *p != target && p.monitor_id() == target.monitor_id())
                .cloned()
                .collect(),
            crate::shared::config::domain::PopupBehavior::Global => active_popups
                .iter()
                .filter(|p| *p != target)
                .cloned()
                .collect(),
        }
    }
}
