use crate::features::module_runtime::domain::Change;
use crate::features::vdom::domain::{UiAction, UiCommand};
use crate::shared::primitives::geometry::Position;
use crate::shared::primitives::{FunctionName, MonitorId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::features::module_runtime) enum PointerAction {
    CallFunction(FunctionName, Option<MonitorId>),
    SendUi(UiCommand),
}

impl PointerAction {
    /// What the actor does for `action` triggered at `pos` on `monitor_id`: a script call on that
    /// monitor, or a UI command, a tray action carrying `pos`.
    #[must_use]
    pub(crate) fn for_ui_action(action: &UiAction, monitor_id: &MonitorId, pos: Position) -> Self {
        match action {
            UiAction::ScriptCall(func_name) => {
                Self::CallFunction(func_name.clone(), Some(monitor_id.clone()))
            }
            UiAction::SystrayAction { id, action, .. } => Self::SendUi(UiCommand::SystrayAction {
                id: id.clone(),
                action: action.clone(),
                pos: Some(pos),
            }),
            UiAction::Exec(cmd_str) => Self::SendUi(UiCommand::Exec(cmd_str.clone())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::features::module_runtime) struct PointerOutcome {
    actions: Vec<PointerAction>,
    state_change: Change,
}

impl PointerOutcome {
    #[must_use]
    pub(crate) const fn new(actions: Vec<PointerAction>, state_change: Change) -> Self {
        Self {
            actions,
            state_change,
        }
    }

    #[must_use]
    pub(crate) const fn empty() -> Self {
        Self {
            actions: Vec::new(),
            state_change: Change::Unchanged,
        }
    }

    #[must_use]
    pub(crate) fn actions(&self) -> &[PointerAction] {
        &self.actions
    }

    #[must_use]
    pub(crate) const fn has_state_changed(&self) -> bool {
        self.state_change.is_changed()
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn into_actions(self) -> Vec<PointerAction> {
        self.actions
    }
}
