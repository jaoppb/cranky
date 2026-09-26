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
    let conf_m_and_m = PopupExclusivity::compute_conflicts(
        &active,
        &p1_1,
        &[],
        PopupBehavior::PerModuleAndMonitor,
    );
    assert_eq!(conf_m_and_m, Vec::<PopupTarget>::new());

    // 2. PerModule (module_a on screen_b conflicts, but not p1_1 itself)
    let conf_module =
        PopupExclusivity::compute_conflicts(&active, &p1_1, &[], PopupBehavior::PerModule);
    assert_eq!(conf_module, vec![p1_2.clone()]);

    // 3. PerMonitor (module_b on screen_a conflicts, but not p1_1 itself)
    let conf_screen =
        PopupExclusivity::compute_conflicts(&active, &p1_1, &[], PopupBehavior::PerMonitor);
    assert_eq!(conf_screen, vec![p2_1.clone()]);

    // 4. Global (all other active popups conflict)
    let conf_global =
        PopupExclusivity::compute_conflicts(&active, &p1_1, &[], PopupBehavior::Global);
    assert_eq!(conf_global, vec![p1_2, p2_1, p2_2]);
}

#[test]
fn test_compute_conflicts_never_returns_an_ancestor() {
    // Decision 7: opening a nested popup must never dismiss the popup it
    // lives inside, under any PopupBehavior — even Global, which otherwise
    // conflicts with everything else active.
    let module_a = ModuleId::new(1);
    let module_b = ModuleId::new(2);
    let screen = MonitorId::new("DP-1");

    let parent = PopupTarget::new(module_a, screen.clone());
    let child = PopupTarget::new(module_b, screen.clone());
    let unrelated = PopupTarget::new(ModuleId::new(3), screen);

    let active = vec![parent.clone(), unrelated.clone()];
    let ancestors = vec![parent.clone()];

    for behavior in [
        PopupBehavior::PerModuleAndMonitor,
        PopupBehavior::PerModule,
        PopupBehavior::PerMonitor,
        PopupBehavior::Global,
    ] {
        let conflicts =
            PopupExclusivity::compute_conflicts(&active, &child, &ancestors, behavior);
        assert!(
            !conflicts.contains(&parent),
            "{behavior:?} must never return an ancestor"
        );
    }

    // Global still catches the unrelated, non-ancestor popup.
    let conflicts =
        PopupExclusivity::compute_conflicts(&active, &child, &ancestors, PopupBehavior::Global);
    assert_eq!(conflicts, vec![unrelated]);
}

#[test]
fn test_ancestors_walks_the_chain_nearest_first() {
    let a = FloatingKind::Popup(PopupTarget::new(ModuleId::new(1), MonitorId::new("DP-1")));
    let b = FloatingKind::Popup(PopupTarget::new(ModuleId::new(2), MonitorId::new("DP-1")));
    let c = FloatingKind::Popup(PopupTarget::new(ModuleId::new(3), MonitorId::new("DP-1")));

    // c's parent is b, b's parent is a, a has no parent (topmost).
    let links = [(c.clone(), Some(b.clone())), (b.clone(), Some(a.clone()))];
    let parent_of = |k: &FloatingKind| links.iter().find(|(kind, _)| kind == k).and_then(|(_, p)| p.clone());

    let result = ancestors(&c, parent_of).expect("no cycle");
    assert_eq!(result, vec![b, a]);
}

#[test]
fn test_ancestors_rejects_a_cycle() {
    let a = FloatingKind::Popup(PopupTarget::new(ModuleId::new(1), MonitorId::new("DP-1")));
    let b = FloatingKind::Popup(PopupTarget::new(ModuleId::new(2), MonitorId::new("DP-1")));

    // a's parent is b, b's parent is a: a cycle.
    let links = [(a.clone(), Some(b.clone())), (b, Some(a.clone()))];
    let parent_of = |k: &FloatingKind| links.iter().find(|(kind, _)| kind == k).and_then(|(_, p)| p.clone());

    assert_eq!(ancestors(&a, parent_of), Err(PopupCycle));
}

#[test]
fn test_descendants_deepest_first_orders_a_three_level_chain() {
    let root = FloatingKind::Popup(PopupTarget::new(ModuleId::new(1), MonitorId::new("DP-1")));
    let mid = FloatingKind::Popup(PopupTarget::new(ModuleId::new(2), MonitorId::new("DP-1")));
    let leaf = FloatingKind::Popup(PopupTarget::new(ModuleId::new(3), MonitorId::new("DP-1")));
    let unrelated = FloatingKind::Popup(PopupTarget::new(ModuleId::new(4), MonitorId::new("DP-1")));

    let links = vec![
        (mid.clone(), Some(root.clone())),
        (leaf.clone(), Some(mid.clone())),
        (unrelated, None),
    ];

    let result = descendants_deepest_first(&root, &links);
    assert_eq!(result, vec![leaf, mid]);
}
