use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum AppCommand {
    RequestRender,
    Exec(String),
    SystrayAction {
        id: crate::features::systray::domain::SystrayId,
        action: crate::features::systray::domain::SystrayActionName,
        #[serde(default)]
        pos: Option<crate::shared::primitives::geometry::Position>,
    },
    ModuleSizeChanged(
        crate::shared::primitives::MonitorId,
        crate::shared::primitives::ModuleId,
        crate::shared::primitives::geometry::Size,
    ),
    ChildModuleSizeChanged {
        parent_id: crate::shared::primitives::ModuleId,
        child_key: crate::shared::primitives::ModuleKey,
        monitor_id: crate::shared::primitives::MonitorId,
        size: crate::shared::primitives::geometry::Size,
    },
    #[serde(skip_deserializing)]
    ContainerLayoutsCalculated {
        parent_id: crate::shared::primitives::ModuleId,
        monitor_id: crate::shared::primitives::MonitorId,
        layouts: Vec<crate::shared::primitives::ChildModuleLayout>,
    },
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
    fn test_app_command_systray_action_deserialization_without_pos() {
        let json = r#"{"SystrayAction": {"id": "test_systray", "action": "Primary"}}"#;
        let cmd: AppCommand = serde_json::from_str(json).unwrap();
        match cmd {
            AppCommand::SystrayAction { id, action, pos } => {
                assert_eq!(id.as_str(), "test_systray");
                assert_eq!(
                    action,
                    crate::features::systray::domain::SystrayActionName::Primary
                );
                assert!(pos.is_none());
            }
            _ => panic!("Expected SystrayAction"),
        }
    }

    #[test]
    fn test_app_command_systray_action_deserialization_with_pos() {
        let json = r#"{"SystrayAction": {"id": "test_systray", "action": "ContextMenu", "pos": {"x": 100, "y": 200}}}"#;
        let cmd: AppCommand = serde_json::from_str(json).unwrap();
        match cmd {
            AppCommand::SystrayAction { id, action, pos } => {
                assert_eq!(id.as_str(), "test_systray");
                assert_eq!(
                    action,
                    crate::features::systray::domain::SystrayActionName::ContextMenu
                );
                assert_eq!(
                    pos,
                    Some(crate::shared::primitives::geometry::Position::new(100, 200))
                );
            }
            _ => panic!("Expected SystrayAction"),
        }
    }
}
