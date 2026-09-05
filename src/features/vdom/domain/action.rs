use crate::shared::events::core::PointerButton;
use crate::shared::primitives::geometry::Position;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum UiAction {
    Exec(String),
    SystrayAction {
        id: crate::features::systray::domain::SystrayId,
        action: crate::features::systray::domain::SystrayActionName,
        #[serde(default)]
        pos: Option<Position>,
    },
    ScriptCall(crate::shared::primitives::FunctionName),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiCommand {
    Exec(String),
    SystrayAction {
        id: crate::features::systray::domain::SystrayId,
        action: crate::features::systray::domain::SystrayActionName,
        pos: Option<Position>,
    },
}

pub trait UiCommandSender: Send + Sync {
    fn send_ui_command(&self, cmd: UiCommand);
}

impl<F> UiCommandSender for F
where
    F: Fn(UiCommand) + Send + Sync,
{
    fn send_ui_command(&self, cmd: UiCommand) {
        self(cmd);
    }
}

impl UiCommandSender for tokio::sync::mpsc::Sender<UiCommand> {
    fn send_ui_command(&self, cmd: UiCommand) {
        if let Err(e) = self.try_send(cmd) {
            tracing::error!(?e, "failed to send ui command via tokio channel");
        }
    }
}

impl UiCommandSender for std::sync::mpsc::Sender<UiCommand> {
    fn send_ui_command(&self, cmd: UiCommand) {
        if let Err(e) = self.send(cmd) {
            tracing::error!(?e, "failed to send ui command via std channel");
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClickHandlers {
    handlers: HashMap<PointerButton, UiAction>,
}

impl ClickHandlers {
    #[must_use]
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    #[must_use]
    pub fn from_single(action: UiAction) -> Self {
        let mut handlers = HashMap::new();
        handlers.insert(PointerButton::Left, action.clone());
        handlers.insert(PointerButton::Middle, action.clone());
        handlers.insert(PointerButton::Right, action);
        Self { handlers }
    }

    #[must_use]
    pub fn get(&self, button: &PointerButton) -> Option<&UiAction> {
        self.handlers.get(button)
    }

    pub fn insert(&mut self, button: PointerButton, action: UiAction) {
        self.handlers.insert(button, action);
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&PointerButton, &UiAction)> {
        self.handlers.iter()
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ClickHandlersHelper {
    Single(UiAction),
    Map(HashMap<String, UiAction>),
}

impl<'de> Deserialize<'de> for ClickHandlers {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match ClickHandlersHelper::deserialize(deserializer)? {
            ClickHandlersHelper::Single(action) => Ok(Self::from_single(action)),
            ClickHandlersHelper::Map(map) => {
                let mut handlers = HashMap::new();
                for (k, v) in map {
                    if let Some(btn) = PointerButton::from_name(&k) {
                        handlers.insert(btn, v);
                    }
                }
                Ok(Self { handlers })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_click_handlers_operations() {
        let mut handlers = ClickHandlers::new();
        assert!(handlers.is_empty());
        assert_eq!(handlers.get(&PointerButton::Left), None);

        handlers.insert(PointerButton::Left, UiAction::Exec("left_cmd".into()));
        assert!(!handlers.is_empty());
        assert_eq!(
            handlers.get(&PointerButton::Left),
            Some(&UiAction::Exec("left_cmd".into()))
        );
        assert_eq!(handlers.get(&PointerButton::Right), None);

        let single = ClickHandlers::from_single(UiAction::Exec("single_cmd".into()));
        assert_eq!(
            single.get(&PointerButton::Left),
            Some(&UiAction::Exec("single_cmd".into()))
        );
        assert_eq!(
            single.get(&PointerButton::Middle),
            Some(&UiAction::Exec("single_cmd".into()))
        );
        assert_eq!(
            single.get(&PointerButton::Right),
            Some(&UiAction::Exec("single_cmd".into()))
        );
        assert_eq!(single.get(&PointerButton::Side), None);
    }

    #[test]
    fn test_click_handlers_deserialization() {
        let single_json = r#"{"Exec": "single_cmd"}"#;
        let single_handlers: ClickHandlers = serde_json::from_str(single_json).unwrap();
        assert_eq!(
            single_handlers.get(&PointerButton::Left),
            Some(&UiAction::Exec("single_cmd".into()))
        );
        assert_eq!(
            single_handlers.get(&PointerButton::Right),
            Some(&UiAction::Exec("single_cmd".into()))
        );

        let map_json = r#"{
            "left": {"Exec": "left_cmd"},
            "right": {"Exec": "right_cmd"},
            "middle": {"Exec": "mid_cmd"},
            "side": {"Exec": "side_cmd"},
            "276": {"Exec": "extra_cmd"}
        }"#;
        let map_handlers: ClickHandlers = serde_json::from_str(map_json).unwrap();
        assert_eq!(
            map_handlers.get(&PointerButton::Left),
            Some(&UiAction::Exec("left_cmd".into()))
        );
        assert_eq!(
            map_handlers.get(&PointerButton::Right),
            Some(&UiAction::Exec("right_cmd".into()))
        );
        assert_eq!(
            map_handlers.get(&PointerButton::Middle),
            Some(&UiAction::Exec("mid_cmd".into()))
        );
        assert_eq!(
            map_handlers.get(&PointerButton::Side),
            Some(&UiAction::Exec("side_cmd".into()))
        );
        assert_eq!(
            map_handlers.get(&PointerButton::Extra),
            Some(&UiAction::Exec("extra_cmd".into()))
        );
    }
}
