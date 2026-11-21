use crate::features::vdom::domain::UiCommand;
use crate::shared::primitives::{FunctionName, MonitorId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::features::module_runtime) enum PointerAction {
    CallFunction(FunctionName, Option<MonitorId>),
    SendUi(UiCommand),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::features::module_runtime) struct PointerOutcome {
    actions: Vec<PointerAction>,
    state_changed: bool,
}

impl PointerOutcome {
    #[must_use]
    pub(crate) const fn new(actions: Vec<PointerAction>, state_changed: bool) -> Self {
        Self {
            actions,
            state_changed,
        }
    }

    #[must_use]
    pub(crate) const fn empty() -> Self {
        Self {
            actions: Vec::new(),
            state_changed: false,
        }
    }

    #[must_use]
    pub(crate) fn actions(&self) -> &[PointerAction] {
        &self.actions
    }

    #[must_use]
    pub(crate) const fn has_state_changed(&self) -> bool {
        self.state_changed
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn into_actions(self) -> Vec<PointerAction> {
        self.actions
    }
}
