use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum AppCommand {
    RequestRender,
    Exec(String),
    AppletAction {
        id: crate::features::applets::domain::AppletId,
        action: crate::features::applets::domain::AppletActionName,
        #[serde(default)]
        pos: Option<crate::shared::primitives::geometry::Position>,
    },
    ModuleSizeChanged(
        crate::shared::primitives::MonitorId,
        crate::shared::primitives::ModuleId,
        crate::shared::primitives::geometry::Size,
    ),
    #[serde(skip_deserializing)]
    ShowTooltip {
        layout: Box<crate::features::layout_engine::domain::StyledNode>,
    },
    HideTooltip,
    ReloadModule(crate::shared::primitives::ModuleName),
    ReloadStyle(crate::features::styling::domain::StyleSheetName),
    ScriptCall(crate::shared::primitives::FunctionName),
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
                assert_eq!(
                    action,
                    crate::features::applets::domain::AppletActionName::Primary
                );
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
                assert_eq!(
                    action,
                    crate::features::applets::domain::AppletActionName::ContextMenu
                );
                assert_eq!(
                    pos,
                    Some(crate::shared::primitives::geometry::Position::new(100, 200))
                );
            }
            _ => panic!("Expected AppletAction"),
        }
    }
}
