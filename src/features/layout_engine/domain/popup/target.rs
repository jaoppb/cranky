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
    /// `ancestors` are `target`'s own ancestor chain (decision 7): opening a
    /// nested popup must never dismiss the popup it lives inside, under any
    /// `PopupBehavior`, so they're excluded from every arm up front rather
    /// than repeating the exclusion four times.
    #[must_use]
    pub fn compute_conflicts(
        active_popups: &[PopupTarget],
        target: &PopupTarget,
        ancestors: &[PopupTarget],
        behavior: crate::shared::config::domain::PopupBehavior,
    ) -> Vec<PopupTarget> {
        let candidates = active_popups
            .iter()
            .filter(|popup| *popup != target && !ancestors.contains(popup));
        match behavior {
            crate::shared::config::domain::PopupBehavior::PerModuleAndMonitor => candidates
                .filter(|popup| {
                    popup.module_id() == target.module_id()
                        && popup.monitor_id() == target.monitor_id()
                })
                .cloned()
                .collect(),
            crate::shared::config::domain::PopupBehavior::PerModule => candidates
                .filter(|popup| popup.module_id() == target.module_id())
                .cloned()
                .collect(),
            crate::shared::config::domain::PopupBehavior::PerMonitor => candidates
                .filter(|popup| popup.monitor_id() == target.monitor_id())
                .cloned()
                .collect(),
            crate::shared::config::domain::PopupBehavior::Global => candidates.cloned().collect(),
        }
    }
}
