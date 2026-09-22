#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::features::workspaces::adapters::hyprland::HyprlandAdapter;
    use crate::features::workspaces::domain::{
        Monitor, MonitorName, Workspace, WorkspaceId, WorkspaceName,
    };
    use crate::shared::events::core::WindowManagerEvent;
    use crate::shared::events::signals::hyprland::HyprlandState;

    #[test]
    fn test_hyprland_state_apply_event() {
        let mut state = HyprlandState::new(BTreeMap::new(), BTreeMap::new(), None);

        // Test WorkspaceCreated
        state.apply_event(&WindowManagerEvent::WorkspaceCreated {
            id: WorkspaceId::new(1),
            name: WorkspaceName::new("1"),
        });
        assert!(state.workspaces().contains_key(&WorkspaceId::new(1)));

        // Test WorkspaceActivated
        state.apply_event(&WindowManagerEvent::WorkspaceActivated {
            id: WorkspaceId::new(2),
            name: WorkspaceName::new("2"),
        });
        assert!(state.workspaces().contains_key(&WorkspaceId::new(2)));

        // Test WorkspaceDestroyed
        state.apply_event(&WindowManagerEvent::WorkspaceDestroyed {
            id: WorkspaceId::new(1),
            name: WorkspaceName::new("1"),
        });
        assert!(!state.workspaces().contains_key(&WorkspaceId::new(1)));

        // Test WorkspaceMoved
        state.apply_event(&WindowManagerEvent::WorkspaceMoved {
            id: WorkspaceId::new(2),
            name: WorkspaceName::new("2"),
            monitor_name: MonitorName::new("DP-1"),
        });
        assert_eq!(
            state
                .workspaces()
                .get(&WorkspaceId::new(2))
                .unwrap()
                .monitor(),
            Some(&MonitorName::new("DP-1"))
        );

        // Test WorkspaceMoved (new workspace)
        state.apply_event(&WindowManagerEvent::WorkspaceMoved {
            id: WorkspaceId::new(3),
            name: WorkspaceName::new("3"),
            monitor_name: MonitorName::new("DP-2"),
        });
        assert_eq!(
            state
                .workspaces()
                .get(&WorkspaceId::new(3))
                .unwrap()
                .monitor(),
            Some(&MonitorName::new("DP-2"))
        );

        // Test WorkspaceRenamed
        state.apply_event(&WindowManagerEvent::WorkspaceRenamed {
            id: WorkspaceId::new(2),
            new_name: WorkspaceName::new("2-renamed"),
        });
        assert!(state.workspaces().contains_key(&WorkspaceId::new(2)));

        // Test MonitorFocused
        let mut monitors = BTreeMap::new();
        monitors.insert(
            MonitorName::new("DP-1"),
            Monitor::new(MonitorName::new("DP-1"), WorkspaceId::new(1), None),
        );
        let mut state = HyprlandState::new(state.workspaces().clone(), monitors, None);
        state.apply_event(&WindowManagerEvent::MonitorFocused {
            monitor_name: MonitorName::new("DP-1"),
            workspace_id: WorkspaceId::new(2),
        });
        assert_eq!(state.focused_monitor(), Some(&MonitorName::new("DP-1")));
        assert_eq!(
            state
                .monitors()
                .get(&MonitorName::new("DP-1"))
                .unwrap()
                .active_workspace_id(),
            &WorkspaceId::new(2)
        );

        // Test SpecialWorkspaceActivated
        state.apply_event(&WindowManagerEvent::SpecialWorkspaceActivated {
            id: Some(WorkspaceId::new(99)),
            name: Some(WorkspaceName::new("special")),
            monitor_name: MonitorName::new("DP-1"),
        });
        assert_eq!(
            state
                .monitors()
                .get(&MonitorName::new("DP-1"))
                .unwrap()
                .special_workspace_id(),
            Some(&WorkspaceId::new(99))
        );
        assert_eq!(
            state
                .workspaces()
                .get(&WorkspaceId::new(99))
                .unwrap()
                .monitor(),
            Some(&MonitorName::new("DP-1"))
        );
    }

    #[test]
    fn test_monitor_removed() {
        let mut monitors = BTreeMap::new();
        monitors.insert(
            MonitorName::new("HDMI-A-1"),
            Monitor::new(MonitorName::new("HDMI-A-1"), WorkspaceId::new(1), None),
        );
        monitors.insert(
            MonitorName::new("eDP-1"),
            Monitor::new(MonitorName::new("eDP-1"), WorkspaceId::new(2), None),
        );
        let mut state = HyprlandState::new(BTreeMap::new(), monitors, None);

        state.apply_event(&WindowManagerEvent::MonitorRemoved {
            name: MonitorName::new("HDMI-A-1"),
        });

        assert!(!state.monitors().contains_key(&MonitorName::new("HDMI-A-1")));
        assert!(state.monitors().contains_key(&MonitorName::new("eDP-1")));
    }

    #[test]
    fn test_monitor_removed_clears_focus_when_it_was_focused() {
        let mut monitors = BTreeMap::new();
        monitors.insert(
            MonitorName::new("HDMI-A-1"),
            Monitor::new(MonitorName::new("HDMI-A-1"), WorkspaceId::new(1), None),
        );
        let mut state = HyprlandState::new(
            BTreeMap::new(),
            monitors,
            Some(MonitorName::new("HDMI-A-1")),
        );

        state.apply_event(&WindowManagerEvent::MonitorRemoved {
            name: MonitorName::new("HDMI-A-1"),
        });

        assert_eq!(state.focused_monitor(), None);
    }

    #[test]
    fn test_monitor_removed_leaves_focus_when_a_different_monitor_was_focused() {
        let mut monitors = BTreeMap::new();
        monitors.insert(
            MonitorName::new("HDMI-A-1"),
            Monitor::new(MonitorName::new("HDMI-A-1"), WorkspaceId::new(1), None),
        );
        monitors.insert(
            MonitorName::new("eDP-1"),
            Monitor::new(MonitorName::new("eDP-1"), WorkspaceId::new(2), None),
        );
        let mut state =
            HyprlandState::new(BTreeMap::new(), monitors, Some(MonitorName::new("eDP-1")));

        state.apply_event(&WindowManagerEvent::MonitorRemoved {
            name: MonitorName::new("HDMI-A-1"),
        });

        assert_eq!(state.focused_monitor(), Some(&MonitorName::new("eDP-1")));
    }

    #[test]
    fn test_monitor_added_is_a_no_op() {
        let mut state = HyprlandState::new(BTreeMap::new(), BTreeMap::new(), None);

        // MonitorAdded carries no active-workspace data, so applying it directly is a
        // no-op - signal_loop treats it as an unconditional resync trigger instead.
        state.apply_event(&WindowManagerEvent::MonitorAdded {
            name: MonitorName::new("HDMI-A-1"),
        });

        assert!(state.monitors().is_empty());
    }

    #[test]
    fn test_monitor_focused_lazily_creates_unknown_monitor() {
        let mut state = HyprlandState::new(BTreeMap::new(), BTreeMap::new(), None);

        // Kept as a fast path for the case where MonitorFocused happens to arrive for a
        // name not yet known - not the primary mechanism monitor-add correctness rests
        // on (see MonitorAdded's forced resync in signal_loop.rs).
        state.apply_event(&WindowManagerEvent::MonitorFocused {
            monitor_name: MonitorName::new("HDMI-A-1"),
            workspace_id: WorkspaceId::new(4),
        });

        assert_eq!(state.focused_monitor(), Some(&MonitorName::new("HDMI-A-1")));
        let mon = state
            .monitors()
            .get(&MonitorName::new("HDMI-A-1"))
            .expect("monitor should have been lazily created");
        assert_eq!(mon.active_workspace_id(), &WorkspaceId::new(4));
        assert_eq!(mon.special_workspace_id(), None);
    }

    #[test]
    fn test_effective_focused_monitor() {
        let mut monitors = BTreeMap::new();
        monitors.insert(
            MonitorName::new("DP-1"),
            Monitor::new(MonitorName::new("DP-1"), WorkspaceId::new(1), None),
        );

        // 1. None focused, single monitor -> returns DP-1
        let state1 = HyprlandState::new(BTreeMap::new(), monitors.clone(), None);
        assert_eq!(
            state1.effective_focused_monitor(),
            Some(MonitorName::new("DP-1"))
        );

        // 2. Focused explicitly set -> returns focused
        let state2 = HyprlandState::new(
            BTreeMap::new(),
            monitors.clone(),
            Some(MonitorName::new("DP-2")),
        );
        assert_eq!(
            state2.effective_focused_monitor(),
            Some(MonitorName::new("DP-2"))
        );

        // 3. None focused, multiple monitors -> returns None
        monitors.insert(
            MonitorName::new("HDMI-1"),
            Monitor::new(MonitorName::new("HDMI-1"), WorkspaceId::new(2), None),
        );
        let state3 = HyprlandState::new(BTreeMap::new(), monitors, None);
        assert_eq!(state3.effective_focused_monitor(), None);
    }

    #[test]
    fn test_workspace_creation_and_activation_with_monitor_inference() {
        let mut monitors = BTreeMap::new();
        monitors.insert(
            MonitorName::new("HEADLESS-1"),
            Monitor::new(MonitorName::new("HEADLESS-1"), WorkspaceId::new(1), None),
        );

        let mut state = HyprlandState::new(
            BTreeMap::new(),
            monitors,
            Some(MonitorName::new("HEADLESS-1")),
        );

        // createworkspacev2>>4,4
        state.apply_event(&WindowManagerEvent::WorkspaceCreated {
            id: WorkspaceId::new(4),
            name: WorkspaceName::new("4"),
        });
        assert_eq!(
            state
                .workspaces()
                .get(&WorkspaceId::new(4))
                .unwrap()
                .monitor(),
            Some(&MonitorName::new("HEADLESS-1"))
        );

        // workspacev2>>4,4
        state.apply_event(&WindowManagerEvent::WorkspaceActivated {
            id: WorkspaceId::new(4),
            name: WorkspaceName::new("4"),
        });
        assert_eq!(
            state
                .monitors()
                .get(&MonitorName::new("HEADLESS-1"))
                .unwrap()
                .active_workspace_id(),
            &WorkspaceId::new(4)
        );

        // Inconsistency validator must pass with zero inconsistencies
        let incs = HyprlandAdapter::find_state_inconsistencies(&state);
        assert!(
            incs.is_empty(),
            "Expected no inconsistencies, got: {incs:?}"
        );
    }

    #[test]
    fn test_hyprland_state_serialize() {
        let mut workspaces = BTreeMap::new();
        workspaces.insert(
            WorkspaceId::new(1),
            Workspace::new(WorkspaceId::new(1), WorkspaceName::new("1"), None),
        );
        let state = HyprlandState::new(workspaces, BTreeMap::new(), None);

        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("\"workspaces\":["));
        assert!(json.contains("\"id\":1"));
    }
}
