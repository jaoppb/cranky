use crate::features::layout_engine::domain::DisplayCommand;
use crate::features::vdom::domain::UiCommand;
use crate::shared::primitives::{FunctionName, MonitorId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PointerAction {
    CallFunction(FunctionName, Option<MonitorId>),
    SendUi(UiCommand),
    SendDisplay(DisplayCommand),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PointerOutcome {
    actions: Vec<PointerAction>,
    state_changed: bool,
}

impl PointerOutcome {
    #[must_use]
    pub const fn new(actions: Vec<PointerAction>, state_changed: bool) -> Self {
        Self {
            actions,
            state_changed,
        }
    }

    #[must_use]
    pub const fn empty() -> Self {
        Self {
            actions: Vec::new(),
            state_changed: false,
        }
    }

    #[must_use]
    pub fn actions(&self) -> &[PointerAction] {
        &self.actions
    }

    #[must_use]
    pub const fn has_state_changed(&self) -> bool {
        self.state_changed
    }

    #[must_use]
    pub fn into_actions(self) -> Vec<PointerAction> {
        self.actions
    }
}
