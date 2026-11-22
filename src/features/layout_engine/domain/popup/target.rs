//! Identities of floating surfaces and the rule for which open popups a new one closes.
use crate::shared::primitives::{ModuleId, MonitorId};
use serde::{Deserialize, Serialize};

/// Identifies an open popup or panel by the module that owns it and the monitor it shows on.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PopupTarget {
    module_id: ModuleId,
    monitor_id: MonitorId,
}

impl PopupTarget {
    #[must_use]
    pub(crate) const fn new(module_id: ModuleId, monitor_id: MonitorId) -> Self {
        Self {
            module_id,
            monitor_id,
        }
    }

    #[must_use]
    pub(crate) const fn module_id(&self) -> ModuleId {
        self.module_id
    }

    #[must_use]
    pub(crate) const fn monitor_id(&self) -> &MonitorId {
        &self.monitor_id
    }
}

/// Identifies one floating surface, the key under which it is shown, nested and hidden.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FloatingKind {
    /// The single tooltip surface.
    Tooltip,
    /// The popup owned by the target module on the target monitor.
    Popup(PopupTarget),
    /// The panel owned by the target module on the target monitor.
    Panel(PopupTarget),
}

/// A popup being opened and its own ancestor chain (decision 7), nearest first: opening it
/// never closes itself nor any popup it lives inside.
#[derive(Debug, Clone, Copy)]
pub(crate) struct OpeningPopup<'a> {
    target: &'a PopupTarget,
    ancestors: &'a [PopupTarget],
}

impl<'a> OpeningPopup<'a> {
    #[must_use]
    pub(crate) const fn new(target: &'a PopupTarget, ancestors: &'a [PopupTarget]) -> Self {
        Self { target, ancestors }
    }
}

/// Decides which open popups a newly opened one closes, per the configured `PopupBehavior`.
#[derive(Debug)]
pub struct PopupExclusivity;

impl PopupExclusivity {
    /// `opening`'s ancestors are its own ancestor chain (decision 7): opening a
    /// nested popup must never dismiss the popup it lives inside, under any
    /// `PopupBehavior`, so they're excluded from every arm up front rather
    /// than repeating the exclusion four times.
    #[must_use]
    pub(crate) fn compute_conflicts(
        active_popups: &[PopupTarget],
        opening: OpeningPopup<'_>,
        behavior: crate::shared::config::domain::PopupBehavior,
    ) -> Vec<PopupTarget> {
        let OpeningPopup { target, ancestors } = opening;
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
