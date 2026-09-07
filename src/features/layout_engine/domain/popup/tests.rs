use super::*;
use crate::shared::config::domain::PopupBehavior;
use crate::shared::primitives::{ModuleId, MonitorId};

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

    let active = vec![p1_1.clone(), p1_2.clone(), p2_1.clone(), p2_2.clone()];

    // 1. PerModuleAndMonitor (no other popups for module_a on screen_a)
    let conf_m_and_m =
        PopupExclusivity::compute_conflicts(&active, &p1_1, PopupBehavior::PerModuleAndMonitor);
    assert_eq!(conf_m_and_m, Vec::<PopupTarget>::new());

    // 2. PerModule (module_a on screen_b conflicts, but not p1_1 itself)
    let conf_module = PopupExclusivity::compute_conflicts(&active, &p1_1, PopupBehavior::PerModule);
    assert_eq!(conf_module, vec![p1_2.clone()]);

    // 3. PerMonitor (module_b on screen_a conflicts, but not p1_1 itself)
    let conf_screen = PopupExclusivity::compute_conflicts(&active, &p1_1, PopupBehavior::PerMonitor);
    assert_eq!(conf_screen, vec![p2_1.clone()]);

    // 4. Global (all other active popups conflict)
    let conf_global = PopupExclusivity::compute_conflicts(&active, &p1_1, PopupBehavior::Global);
    assert_eq!(conf_global, vec![p1_2, p2_1, p2_2]);
}
