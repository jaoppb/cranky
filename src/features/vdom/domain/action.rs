//! What a node does when clicked or hovered, and the UI commands those actions send to the app.
use crate::shared::events::core::PointerButton;
use crate::shared::primitives::geometry::Position;
use serde::Deserialize;
use std::collections::HashMap;

/// What a script asked to happen when a node is clicked or hovered, as built with `ui.action`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum UiAction {
    /// Runs the string as a shell command with `sh -c`.
    Exec(String),
    /// Asks a tray item to perform an action.
    SystrayAction {
        /// The tray item to act on.
        id: crate::features::systray::domain::SystrayId,
        /// Which `StatusNotifierItem` method, or fallback chain, to invoke on it.
        action: crate::features::systray::domain::SystrayActionName,
        /// Ignored when dispatched: the pointer position of the event is sent instead.
        #[serde(default)]
        pos: Option<Position>,
    },
    /// Calls the named function in the module's own script, with the monitor name as its argument.
    ScriptCall(crate::shared::primitives::FunctionName),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UiCommand {
    Exec(String),
    SystrayAction {
        id: crate::features::systray::domain::SystrayId,
        action: crate::features::systray::domain::SystrayActionName,
        pos: Option<Position>,
    },
}

pub(crate) trait UiCommandSender: Send + Sync {
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
        if let Err(error) = self.try_send(cmd) {
            log_tokio_send_failed(&error);
        }
    }
}

/// Logs a command the tokio channel refused (full or closed).
fn log_tokio_send_failed(error: &tokio::sync::mpsc::error::TrySendError<UiCommand>) {
    tracing::error!(?error, "failed to send ui command via tokio channel");
}

impl UiCommandSender for std::sync::mpsc::Sender<UiCommand> {
    fn send_ui_command(&self, cmd: UiCommand) {
        if let Err(error) = self.send(cmd) {
            log_std_send_failed(&error);
        }
    }
}

/// Logs a command the std channel refused because its receiver is gone.
fn log_std_send_failed(error: &std::sync::mpsc::SendError<UiCommand>) {
    tracing::error!(?error, "failed to send ui command via std channel");
}

/// The action bound to each pointer button on a node; a single action from a script binds left,
/// middle and right alike.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClickHandlers {
    handlers: HashMap<PointerButton, UiAction>,
}

impl ClickHandlers {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    #[must_use]
    pub(crate) fn from_single(action: UiAction) -> Self {
        let mut handlers = HashMap::new();
        handlers.insert(PointerButton::Left, action.clone());
        handlers.insert(PointerButton::Middle, action.clone());
        handlers.insert(PointerButton::Right, action);
        Self { handlers }
    }

    #[must_use]
    pub(crate) fn get(&self, button: PointerButton) -> Option<&UiAction> {
        self.handlers.get(&button)
    }

    pub(crate) fn insert(&mut self, button: PointerButton, action: UiAction) {
        self.handlers.insert(button, action);
    }

    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.handlers.is_empty()
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
                for (name, action) in map {
                    if let Some(btn) = PointerButton::from_name(&name) {
                        handlers.insert(btn, action);
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
        assert_eq!(handlers.get(PointerButton::Left), None);

        handlers.insert(PointerButton::Left, UiAction::Exec("left_cmd".into()));
        assert!(!handlers.is_empty());
        assert_eq!(
            handlers.get(PointerButton::Left),
            Some(&UiAction::Exec("left_cmd".into()))
        );
        assert_eq!(handlers.get(PointerButton::Right), None);

        let single = ClickHandlers::from_single(UiAction::Exec("single_cmd".into()));
        assert_eq!(
            single.get(PointerButton::Left),
            Some(&UiAction::Exec("single_cmd".into()))
        );
        assert_eq!(
            single.get(PointerButton::Middle),
            Some(&UiAction::Exec("single_cmd".into()))
        );
        assert_eq!(
            single.get(PointerButton::Right),
            Some(&UiAction::Exec("single_cmd".into()))
        );
        assert_eq!(single.get(PointerButton::Side), None);
    }

    #[test]
    fn test_click_handlers_deserialization() {
        let single_json = r#"{"Exec": "single_cmd"}"#;
        let single_handlers: ClickHandlers = serde_json::from_str(single_json).unwrap();
        assert_eq!(
            single_handlers.get(PointerButton::Left),
            Some(&UiAction::Exec("single_cmd".into()))
        );
        assert_eq!(
            single_handlers.get(PointerButton::Right),
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
            map_handlers.get(PointerButton::Left),
            Some(&UiAction::Exec("left_cmd".into()))
        );
        assert_eq!(
            map_handlers.get(PointerButton::Right),
            Some(&UiAction::Exec("right_cmd".into()))
        );
        assert_eq!(
            map_handlers.get(PointerButton::Middle),
            Some(&UiAction::Exec("mid_cmd".into()))
        );
        assert_eq!(
            map_handlers.get(PointerButton::Side),
            Some(&UiAction::Exec("side_cmd".into()))
        );
        assert_eq!(
            map_handlers.get(PointerButton::Extra),
            Some(&UiAction::Exec("extra_cmd".into()))
        );
    }

    /// The `Exec` command for `command`, as a sender would receive it.
    fn exec(command: &str) -> UiCommand {
        UiCommand::Exec(command.to_string())
    }

    /// A shared byte buffer a test subscriber writes its formatted events into.
    #[derive(Clone, Default)]
    struct LogBuffer(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

    impl std::io::Write for LogBuffer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .map_err(|_| std::io::Error::other("log buffer poisoned"))?
                .extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// Everything logged while `action` runs, formatted without time or colour.
    fn captured_logs(action: impl FnOnce()) -> String {
        let buffer = LogBuffer::default();
        let writer = buffer.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(move || writer.clone())
            .with_ansi(false)
            .without_time()
            .finish();
        tracing::subscriber::with_default(subscriber, action);
        let bytes = buffer.0.lock().unwrap().clone();
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn test_closure_sender_forwards_the_command() {
        let received = std::sync::Mutex::new(Vec::new());
        let sender = |cmd: UiCommand| received.lock().unwrap().push(cmd);
        sender.send_ui_command(exec("run"));
        assert_eq!(received.into_inner().unwrap(), vec![exec("run")]);
    }

    #[test]
    fn test_tokio_sender_delivers_without_logging() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let logs = captured_logs(|| tx.send_ui_command(exec("run")));
        assert_eq!(
            (rx.try_recv().ok(), logs),
            (Some(exec("run")), String::new())
        );
    }

    #[test]
    fn test_tokio_sender_logs_and_drops_the_command_when_the_channel_is_full() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        tx.send_ui_command(exec("first"));
        let logs = captured_logs(|| tx.send_ui_command(exec("second")));
        assert_eq!(
            logs,
            "ERROR cranky::features::vdom::domain::action: failed to send ui command via tokio \
             channel error=\"Full(..)\"\n"
        );
        assert_eq!(rx.try_recv().ok(), Some(exec("first")));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn test_std_sender_delivers_without_logging() {
        let (tx, rx) = std::sync::mpsc::channel();
        let logs = captured_logs(|| tx.send_ui_command(exec("run")));
        assert_eq!(
            (rx.try_recv().ok(), logs),
            (Some(exec("run")), String::new())
        );
    }

    #[test]
    fn test_std_sender_logs_when_the_receiver_is_gone() {
        let (tx, rx) = std::sync::mpsc::channel();
        drop(rx);
        let logs = captured_logs(|| tx.send_ui_command(exec("run")));
        assert_eq!(
            logs,
            "ERROR cranky::features::vdom::domain::action: failed to send ui command via std \
             channel error=SendError { .. }\n"
        );
    }

    #[test]
    fn test_click_handlers_deserialization_drops_unknown_button_names() {
        let json = r#"{"left": {"Exec": "left_cmd"}, "wheel": {"Exec": "wheel_cmd"}}"#;
        let handlers: ClickHandlers = serde_json::from_str(json).unwrap();
        let mut expected = ClickHandlers::new();
        expected.insert(PointerButton::Left, UiAction::Exec("left_cmd".into()));
        assert_eq!(handlers, expected);
    }
}
