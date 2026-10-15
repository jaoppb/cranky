#[cfg(test)]
mod tests {
    use crate::features::layout_engine::domain::{NodePath, RenderNode, SurfaceSpace};
    use crate::features::module_runtime::domain::pointer_handler::action::{
        PointerAction, PointerOutcome,
    };
    use crate::features::module_runtime::domain::pointer_handler::handler::PointerHandler;
    use crate::features::styling::domain::ComputedStyle;
    use crate::features::vdom::domain::NodeRef;
    use crate::shared::events::core::{PointerButton, PointerEvent, SurfaceKind};
    use crate::shared::primitives::geometry::{Position, Rect, Size};
    use crate::shared::primitives::{FunctionName, MonitorId};

    fn make_test_tree() -> RenderNode {
        RenderNode::Rect {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            rect: Rect::new(Position::new(0, 0), Size::new(100, 100)),
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        }
    }

    fn root_ref() -> NodeRef {
        NodeRef::new(Vec::new(), NodePath::root_in(SurfaceSpace::Bar))
    }

    #[test]
    fn test_interaction_state_lifecycle() {
        let mut handler = PointerHandler::new();
        let mon = MonitorId::new("DP-1");
        let tree = make_test_tree();

        let outcome_motion = handler.handle_event(
            &PointerEvent::PointerMotion {
                surface: SurfaceKind::Bar,
                pos: Position::new(10, 10),
            },
            &mon,
            &tree,
        );
        assert!(outcome_motion.has_state_changed());
        assert_eq!(handler.hovered_node(&mon), Some(&root_ref()));

        let outcome_press = handler.handle_event(
            &PointerEvent::ButtonPress {
                surface: SurfaceKind::Bar,
                button: PointerButton::Left,
                pos: Position::new(10, 10),
            },
            &mon,
            &tree,
        );
        assert!(outcome_press.has_state_changed());
        assert_eq!(handler.active_node(&mon), Some(&root_ref()));

        let outcome_release = handler.handle_event(
            &PointerEvent::ButtonRelease {
                surface: SurfaceKind::Bar,
                button: PointerButton::Left,
                pos: Position::new(10, 10),
            },
            &mon,
            &tree,
        );
        assert!(outcome_release.has_state_changed());
        assert_eq!(handler.active_node(&mon), None);
        assert_eq!(handler.focused_node(&mon), Some(&root_ref()));

        let outcome_leave = handler.handle_event(
            &PointerEvent::PointerLeave {
                surface: SurfaceKind::Bar,
            },
            &mon,
            &tree,
        );
        assert!(outcome_leave.has_state_changed());
        assert_eq!(handler.hovered_node(&mon), None);
        assert_eq!(handler.focused_node(&mon), Some(&root_ref()));
    }

    #[test]
    fn test_popup_dismissed_event() {
        let mut handler = PointerHandler::new();
        let mon = MonitorId::new("DP-1");

        let outcome = handler.handle_dismissal(
            crate::shared::events::core::SurfaceLifecycleEvent::PopupDismissed,
            &mon,
        );
        assert!(outcome.has_state_changed());
        assert_eq!(
            outcome.into_actions(),
            vec![PointerAction::CallFunction(
                FunctionName::new("on_popup_dismiss"),
                Some(mon)
            )]
        );
    }

    #[test]
    fn test_panel_dismissed_event() {
        let mut handler = PointerHandler::new();
        let mon = MonitorId::new("DP-1");

        let outcome = handler.handle_dismissal(
            crate::shared::events::core::SurfaceLifecycleEvent::PanelDismissed,
            &mon,
        );
        assert!(outcome.has_state_changed());
        assert_eq!(
            outcome.into_actions(),
            vec![PointerAction::CallFunction(
                FunctionName::new("on_panel_dismiss"),
                Some(mon)
            )]
        );
    }

    #[test]
    fn test_pointer_outcome_methods() {
        let empty = PointerOutcome::empty();
        assert!(empty.actions().is_empty());
        assert!(!empty.has_state_changed());
        assert_eq!(empty.into_actions(), Vec::<PointerAction>::new());
    }

    /// Merge seam: `retain_monitors` must drop a dead monitor's hover/active/
    /// focus state but leave a live monitor's alone, otherwise a reused
    /// monitor id after a hotplug would resurrect stale interaction state.
    #[test]
    fn test_retain_monitors_drops_state_for_dead_monitor_only() {
        let mut handler = PointerHandler::new();
        let dead = MonitorId::new("HDMI-A-1");
        let live = MonitorId::new("eDP-1");
        let tree = make_test_tree();

        for mon in [&dead, &live] {
            let _ = handler.handle_event(
                &PointerEvent::PointerMotion {
                    surface: SurfaceKind::Bar,
                    pos: Position::new(10, 10),
                },
                mon,
                &tree,
            );
        }
        assert!(handler.hovered_node(&dead).is_some());
        assert!(handler.hovered_node(&live).is_some());

        let mut still_live = std::collections::HashSet::new();
        still_live.insert(live.clone());
        handler.retain_monitors(&still_live);

        assert!(handler.hovered_node(&dead).is_none());
        assert!(handler.hovered_node(&live).is_some());
    }
}
