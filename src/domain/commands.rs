use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum AppCommand {
    RequestRender,
    Exec(String),
    AppletAction {
        id: crate::domain::applets::AppletId,
        action: crate::domain::applets::AppletActionName,
        #[serde(default)]
        pos: Option<crate::domain::shared::geometry::Position>,
    },
    ModuleSizeChanged(
        crate::domain::MonitorId,
        crate::domain::ModuleId,
        crate::domain::shared::geometry::Size,
    ),
    ShowTooltip {
        layout: Box<crate::domain::layout::LayoutNode>,
    },
    HideTooltip,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_command_applet_action_deserialization_without_pos() {
        let json = r#"{"AppletAction": {"id": "test_applet", "action": "Primary"}}"#;
        let cmd: AppCommand = serde_json::from_str(json).unwrap();
        match cmd {
            AppCommand::AppletAction { id, action, pos } => {
                assert_eq!(id.as_str(), "test_applet");
                assert_eq!(action, crate::domain::applets::AppletActionName::Primary);
                assert!(pos.is_none());
            }
            _ => panic!("Expected AppletAction"),
        }
    }

    #[test]
    fn test_app_command_applet_action_deserialization_with_pos() {
        let json = r#"{"AppletAction": {"id": "test_applet", "action": "ContextMenu", "pos": {"x": 100, "y": 200}}}"#;
        let cmd: AppCommand = serde_json::from_str(json).unwrap();
        match cmd {
            AppCommand::AppletAction { id, action, pos } => {
                assert_eq!(id.as_str(), "test_applet");
                assert_eq!(action, crate::domain::applets::AppletActionName::ContextMenu);
                assert_eq!(pos, Some(crate::domain::shared::geometry::Position::new(100, 200)));
            }
            _ => panic!("Expected AppletAction"),
        }
    }
}

