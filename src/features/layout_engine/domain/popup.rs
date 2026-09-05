use super::styled_node::StyledNode;
use crate::shared::primitives::geometry::Rect;
use crate::shared::primitives::{ModuleId, MonitorId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PopupTarget {
    module_id: ModuleId,
    monitor_id: MonitorId,
}

impl PopupTarget {
    #[must_use]
    pub const fn new(module_id: ModuleId, monitor_id: MonitorId) -> Self {
        Self {
            module_id,
            monitor_id,
        }
    }

    #[must_use]
    pub const fn module_id(&self) -> ModuleId {
        self.module_id
    }

    #[must_use]
    pub const fn monitor_id(&self) -> &MonitorId {
        &self.monitor_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FloatingKind {
    Tooltip,
    Popup(PopupTarget),
}

pub struct PopupExclusivity;

impl PopupExclusivity {
    #[must_use]
    pub fn compute_conflicts(
        active_popups: &[PopupTarget],
        target: &PopupTarget,
        behavior: crate::shared::config::domain::PopupBehavior,
    ) -> Vec<PopupTarget> {
        match behavior {
            crate::shared::config::domain::PopupBehavior::PerModuleAndMonitor => active_popups
                .iter()
                .filter(|p| {
                    p.module_id() == target.module_id() && p.monitor_id() == target.monitor_id()
                })
                .cloned()
                .collect(),
            crate::shared::config::domain::PopupBehavior::PerModule => active_popups
                .iter()
                .filter(|p| p.module_id() == target.module_id())
                .cloned()
                .collect(),
            crate::shared::config::domain::PopupBehavior::PerMonitor => active_popups
                .iter()
                .filter(|p| p.monitor_id() == target.monitor_id())
                .cloned()
                .collect(),
            crate::shared::config::domain::PopupBehavior::Global => active_popups.to_vec(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnchoredPopup<'a> {
    anchor_rect: Rect,
    layout: &'a StyledNode,
}

impl<'a> AnchoredPopup<'a> {
    #[must_use]
    pub const fn new(anchor_rect: Rect, layout: &'a StyledNode) -> Self {
        Self {
            anchor_rect,
            layout,
        }
    }

    #[must_use]
    pub const fn anchor_rect(&self) -> &Rect {
        &self.anchor_rect
    }

    #[must_use]
    pub const fn layout(&self) -> &'a StyledNode {
        self.layout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::config::domain::PopupBehavior;

    #[test]
    fn test_popup_target_and_exclusivity() {
        let module_a = ModuleId::new(1);
        let module_b = ModuleId::new(2);
        let screen_a = MonitorId::new("DP-1");
        let screen_b = MonitorId::new("DP-2");

        let p1_1 = PopupTarget::new(module_a, screen_a.clone());
        let p1_2 = PopupTarget::new(module_a, screen_b.clone());
        let p2_1 = PopupTarget::new(module_b, screen_a.clone());
        let p2_2 = PopupTarget::new(module_b, screen_b);

        assert_eq!(p1_1.module_id(), module_a);
        assert_eq!(p1_1.monitor_id(), &screen_a);

        let active = vec![p1_1.clone(), p1_2.clone(), p2_1.clone(), p2_2];

        // 1. PerModuleAndMonitor
        let conf_m_and_m =
            PopupExclusivity::compute_conflicts(&active, &p1_1, PopupBehavior::PerModuleAndMonitor);
        assert_eq!(conf_m_and_m, vec![p1_1.clone()]);

        // 2. PerModule
        let conf_module =
            PopupExclusivity::compute_conflicts(&active, &p1_1, PopupBehavior::PerModule);
        assert_eq!(conf_module, vec![p1_1.clone(), p1_2]);

        // 3. PerMonitor
        let conf_screen =
            PopupExclusivity::compute_conflicts(&active, &p1_1, PopupBehavior::PerMonitor);
        assert_eq!(conf_screen, vec![p1_1.clone(), p2_1]);

        // 4. Global
        let conf_global =
            PopupExclusivity::compute_conflicts(&active, &p1_1, PopupBehavior::Global);
        assert_eq!(conf_global, active);
    }
}
