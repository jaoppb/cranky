//! Tests for `HyprlandState` event application and serialisation.

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::features::workspaces::adapters::hyprland::HyprlandAdapter;
    use crate::features::workspaces::domain::{
        Monitor, MonitorName, Workspace, WorkspaceId, WorkspaceName,
    };
    use crate::shared::events::core::WindowManagerEvent;
    use crate::shared::events::signals::hyprland::HyprlandState;

    /// The monitor workspace `id` is on, or `None` when it is unknown or on no monitor.
    fn monitor_of(state: &HyprlandState, id: i32) -> Option<&MonitorName> {
        state.workspaces().get(&WorkspaceId::new(id))?.monitor()
    }

    /// The workspace `monitor` shows; the monitor must be known.
    fn active_workspace_of<'state>(
        state: &'state HyprlandState,
        monitor: &str,
    ) -> &'state WorkspaceId {
        state
            .monitors()
            .get(&MonitorName::new(monitor))
            .unwrap()
            .active_workspace_id()
    }

    /// The special workspace `monitor` shows, or `None` when it is unknown or shows none.
    fn special_workspace_of<'state>(
        state: &'state HyprlandState,
        monitor: &str,
    ) -> Option<&'state WorkspaceId> {
        state
            .monitors()
            .get(&MonitorName::new(monitor))?
            .special_workspace_id()
    }

    /// The single monitor `name`, showing workspace `active`.
    fn only_monitor(name: &str, active: i32) -> BTreeMap<MonitorName, Monitor> {
        let monitor = Monitor::new(MonitorName::new(name), WorkspaceId::new(active), None);
        BTreeMap::from([(MonitorName::new(name), monitor)])
    }

    /// A state that knows workspace 2 on no monitor, as left by activating it while nothing
    /// had focus.
    fn state_with_workspace_two() -> HyprlandState {
        let mut workspaces = BTreeMap::new();
        workspaces.insert(
            WorkspaceId::new(2),
            Workspace::new(WorkspaceId::new(2), WorkspaceName::new("2"), None),
        );
        HyprlandState::new(workspaces, BTreeMap::new(), None)
    }

    // The create / activate / destroy, move / rename and focus / special steps were one test
    // chained on one state. The first step ends in the state the second starts from; the third
    // starts from the same workspaces with a DP-1 monitor added, not from what the second left.
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
        assert_eq!(state, state_with_workspace_two());
    }

    #[test]
    fn test_hyprland_state_apply_workspace_moved_and_renamed() {
        let mut state = state_with_workspace_two();

        // Test WorkspaceMoved
        state.apply_event(&WindowManagerEvent::WorkspaceMoved {
            id: WorkspaceId::new(2),
            name: WorkspaceName::new("2"),
            monitor_name: MonitorName::new("DP-1"),
        });
        assert_eq!(monitor_of(&state, 2), Some(&MonitorName::new("DP-1")));

        // Test WorkspaceMoved (new workspace)
        state.apply_event(&WindowManagerEvent::WorkspaceMoved {
            id: WorkspaceId::new(3),
            name: WorkspaceName::new("3"),
            monitor_name: MonitorName::new("DP-2"),
        });
        assert_eq!(monitor_of(&state, 3), Some(&MonitorName::new("DP-2")));

        // Test WorkspaceRenamed
        state.apply_event(&WindowManagerEvent::WorkspaceRenamed {
            id: WorkspaceId::new(2),
            new_name: WorkspaceName::new("2-renamed"),
        });
        assert!(state.workspaces().contains_key(&WorkspaceId::new(2)));
    }

    #[test]
    fn test_hyprland_state_apply_monitor_focused_and_special_workspace() {
        let workspaces = state_with_workspace_two().workspaces().clone();
        let mut state = HyprlandState::new(workspaces, only_monitor("DP-1", 1), None);

        // Test MonitorFocused
        state.apply_event(&WindowManagerEvent::MonitorFocused {
            monitor_name: MonitorName::new("DP-1"),
            workspace_id: WorkspaceId::new(2),
        });
        assert_eq!(state.focused_monitor(), Some(&MonitorName::new("DP-1")));
        assert_eq!(active_workspace_of(&state, "DP-1"), &WorkspaceId::new(2));

        // Test SpecialWorkspaceActivated
        state.apply_event(&WindowManagerEvent::SpecialWorkspaceActivated {
            id: Some(WorkspaceId::new(99)),
            name: Some(WorkspaceName::new("special")),
            monitor_name: MonitorName::new("DP-1"),
        });
        assert_eq!(
            special_workspace_of(&state, "DP-1"),
            Some(&WorkspaceId::new(99))
        );
        assert_eq!(monitor_of(&state, 99), Some(&MonitorName::new("DP-1")));
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
        let monitors = only_monitor("HEADLESS-1", 1);
        let focused = Some(MonitorName::new("HEADLESS-1"));
        let mut state = HyprlandState::new(BTreeMap::new(), monitors, focused);

        // createworkspacev2>>4,4
        state.apply_event(&WindowManagerEvent::WorkspaceCreated {
            id: WorkspaceId::new(4),
            name: WorkspaceName::new("4"),
        });
        assert_eq!(monitor_of(&state, 4), Some(&MonitorName::new("HEADLESS-1")));

        // workspacev2>>4,4
        state.apply_event(&WindowManagerEvent::WorkspaceActivated {
            id: WorkspaceId::new(4),
            name: WorkspaceName::new("4"),
        });
        assert_eq!(
            active_workspace_of(&state, "HEADLESS-1"),
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

    fn one_monitor_state(workspaces: BTreeMap<WorkspaceId, Workspace>) -> HyprlandState {
        let mut monitors = BTreeMap::new();
        monitors.insert(
            MonitorName::new("DP-1"),
            Monitor::new(
                MonitorName::new("DP-1"),
                WorkspaceId::new(1),
                Some(WorkspaceId::new(98)),
            ),
        );
        HyprlandState::new(workspaces, monitors, None)
    }

    fn special_activated(id: Option<i32>, name: Option<&str>, monitor: &str) -> WindowManagerEvent {
        WindowManagerEvent::SpecialWorkspaceActivated {
            id: id.map(WorkspaceId::new),
            name: name.map(WorkspaceName::new),
            monitor_name: MonitorName::new(monitor),
        }
    }

    #[test]
    fn test_special_workspace_moves_a_known_workspace_to_its_monitor() {
        let mut workspaces = BTreeMap::new();
        workspaces.insert(
            WorkspaceId::new(99),
            Workspace::new(
                WorkspaceId::new(99),
                WorkspaceName::new("scratch"),
                Some(MonitorName::new("HDMI-A-1")),
            ),
        );
        let mut state = one_monitor_state(workspaces);

        state.apply_event(&special_activated(Some(99), Some("ignored"), "DP-1"));

        let workspace = state.workspaces().get(&WorkspaceId::new(99)).unwrap();
        assert_eq!(workspace.monitor(), Some(&MonitorName::new("DP-1")));
        assert_eq!(workspace.name(), &WorkspaceName::new("scratch"));
        assert_eq!(
            state
                .monitors()
                .get(&MonitorName::new("DP-1"))
                .unwrap()
                .special_workspace_id(),
            Some(&WorkspaceId::new(99))
        );
    }

    #[test]
    fn test_special_workspace_without_a_name_is_not_created() {
        let mut state = one_monitor_state(BTreeMap::new());

        state.apply_event(&special_activated(Some(99), None, "DP-1"));

        assert!(state.workspaces().is_empty());
        assert_eq!(
            state
                .monitors()
                .get(&MonitorName::new("DP-1"))
                .unwrap()
                .special_workspace_id(),
            Some(&WorkspaceId::new(99))
        );
    }

    #[test]
    fn test_special_workspace_closed_clears_the_monitor_slot() {
        let mut state = one_monitor_state(BTreeMap::new());

        state.apply_event(&special_activated(None, None, "DP-1"));

        assert!(state.workspaces().is_empty());
        assert_eq!(
            state
                .monitors()
                .get(&MonitorName::new("DP-1"))
                .unwrap()
                .special_workspace_id(),
            None
        );
    }

    #[test]
    fn test_special_workspace_on_an_unknown_monitor_still_records_the_workspace() {
        let mut state = one_monitor_state(BTreeMap::new());

        state.apply_event(&special_activated(Some(99), Some("magic"), "HDMI-A-1"));

        assert!(!state.monitors().contains_key(&MonitorName::new("HDMI-A-1")));
        assert_eq!(
            state
                .monitors()
                .get(&MonitorName::new("DP-1"))
                .unwrap()
                .special_workspace_id(),
            Some(&WorkspaceId::new(98))
        );
        let workspace = state.workspaces().get(&WorkspaceId::new(99)).unwrap();
        assert_eq!(workspace.name(), &WorkspaceName::new("magic"));
        assert_eq!(workspace.monitor(), Some(&MonitorName::new("HDMI-A-1")));
    }

    #[test]
    fn test_workspace_created_keeps_a_workspace_it_already_knows() {
        let mut workspaces = BTreeMap::new();
        workspaces.insert(
            WorkspaceId::new(4),
            Workspace::new(
                WorkspaceId::new(4),
                WorkspaceName::new("four"),
                Some(MonitorName::new("HDMI-A-1")),
            ),
        );
        let mut state = one_monitor_state(workspaces.clone());

        state.apply_event(&WindowManagerEvent::WorkspaceCreated {
            id: WorkspaceId::new(4),
            name: WorkspaceName::new("renamed"),
        });

        assert_eq!(state.workspaces(), &workspaces);
    }

    #[test]
    fn test_workspace_renamed_ignores_an_unknown_workspace() {
        let mut state = one_monitor_state(BTreeMap::new());

        state.apply_event(&WindowManagerEvent::WorkspaceRenamed {
            id: WorkspaceId::new(7),
            new_name: WorkspaceName::new("seven"),
        });

        assert!(state.workspaces().is_empty());
    }

    #[test]
    fn test_monitor_focused_moves_its_workspace_onto_the_monitor() {
        let mut workspaces = BTreeMap::new();
        workspaces.insert(
            WorkspaceId::new(2),
            Workspace::new(
                WorkspaceId::new(2),
                WorkspaceName::new("two"),
                Some(MonitorName::new("HDMI-A-1")),
            ),
        );
        let mut state = one_monitor_state(workspaces);

        state.apply_event(&WindowManagerEvent::MonitorFocused {
            monitor_name: MonitorName::new("DP-1"),
            workspace_id: WorkspaceId::new(2),
        });

        let workspace = state.workspaces().get(&WorkspaceId::new(2)).unwrap();
        assert_eq!(workspace.monitor(), Some(&MonitorName::new("DP-1")));
        assert_eq!(workspace.name(), &WorkspaceName::new("two"));
    }

    #[test]
    fn test_workspace_renamed_takes_the_new_name_and_keeps_its_monitor() {
        let mut workspaces = BTreeMap::new();
        workspaces.insert(
            WorkspaceId::new(4),
            Workspace::new(
                WorkspaceId::new(4),
                WorkspaceName::new("four"),
                Some(MonitorName::new("HDMI-A-1")),
            ),
        );
        let mut state = one_monitor_state(workspaces);

        state.apply_event(&WindowManagerEvent::WorkspaceRenamed {
            id: WorkspaceId::new(4),
            new_name: WorkspaceName::new("renamed"),
        });

        let workspace = state.workspaces().get(&WorkspaceId::new(4)).unwrap();
        assert_eq!(workspace.name(), &WorkspaceName::new("renamed"));
        assert_eq!(workspace.monitor(), Some(&MonitorName::new("HDMI-A-1")));
    }

    /// The workspace `id` named `name`, on `monitor` when there is one.
    fn workspace(id: i32, name: &str, monitor: Option<&str>) -> Workspace {
        Workspace::new(
            WorkspaceId::new(id),
            WorkspaceName::new(name),
            monitor.map(MonitorName::new),
        )
    }

    /// `WorkspaceActivated` for workspace `id` named `name`.
    fn activated(id: i32, name: &str) -> WindowManagerEvent {
        WindowManagerEvent::WorkspaceActivated {
            id: WorkspaceId::new(id),
            name: WorkspaceName::new(name),
        }
    }

    #[test]
    fn test_workspace_activated_records_an_unknown_workspace_on_the_focused_monitor() {
        let focused = Some(MonitorName::new("DP-1"));
        let mut state = HyprlandState::new(BTreeMap::new(), only_monitor("DP-1", 1), focused);

        state.apply_event(&activated(5, "five"));

        let workspaces =
            BTreeMap::from([(WorkspaceId::new(5), workspace(5, "five", Some("DP-1")))]);
        let expected = HyprlandState::new(
            workspaces,
            only_monitor("DP-1", 5),
            Some(MonitorName::new("DP-1")),
        );
        assert_eq!(state, expected);
    }

    #[test]
    fn test_workspace_activated_on_a_focused_monitor_it_does_not_know_leaves_the_monitors() {
        let focused = Some(MonitorName::new("DP-9"));
        let mut state = HyprlandState::new(BTreeMap::new(), only_monitor("DP-1", 1), focused);

        state.apply_event(&activated(5, "five"));

        let workspaces =
            BTreeMap::from([(WorkspaceId::new(5), workspace(5, "five", Some("DP-9")))]);
        let expected = HyprlandState::new(
            workspaces,
            only_monitor("DP-1", 1),
            Some(MonitorName::new("DP-9")),
        );
        assert_eq!(state, expected);
    }

    #[test]
    fn test_workspace_activated_without_a_focused_monitor_leaves_a_known_workspace() {
        let workspaces =
            BTreeMap::from([(WorkspaceId::new(2), workspace(2, "two", Some("HDMI-A-1")))]);
        let mut state = HyprlandState::new(workspaces, BTreeMap::new(), None);
        let before = state.clone();

        state.apply_event(&activated(2, "renamed"));

        assert_eq!(state, before);
    }
}
