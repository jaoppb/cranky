use super::*;
use crate::shared::config::domain::PopupBehavior;
use crate::shared::primitives::{ModuleId, MonitorId};

#[test]
fn test_popup_target_and_exclusivity() {
    let (module_a, module_b) = (ModuleId::new(1), ModuleId::new(2));
    let (screen_a, screen_b) = (MonitorId::new("DP-1"), MonitorId::new("DP-2"));
    let p1_1 = PopupTarget::new(module_a, screen_a.clone());
    let p1_2 = PopupTarget::new(module_a, screen_b.clone());
    let p2_1 = PopupTarget::new(module_b, screen_a.clone());
    let p2_2 = PopupTarget::new(module_b, screen_b);
    assert_eq!((p1_1.module_id(), p1_1.monitor_id()), (module_a, &screen_a));

    let active = vec![p1_1.clone(), p1_2.clone(), p2_1.clone(), p2_2.clone()];
    let conflicts = |behavior| {
        PopupExclusivity::compute_conflicts(&active, OpeningPopup::new(&p1_1, &[]), behavior)
    };
    // 1. PerModuleAndMonitor (no other popups for module_a on screen_a)
    assert_eq!(
        conflicts(PopupBehavior::PerModuleAndMonitor),
        Vec::<PopupTarget>::new()
    );
    // 2. PerModule (module_a on screen_b conflicts, but not p1_1 itself)
    assert_eq!(conflicts(PopupBehavior::PerModule), vec![p1_2.clone()]);
    // 3. PerMonitor (module_b on screen_a conflicts, but not p1_1 itself)
    assert_eq!(conflicts(PopupBehavior::PerMonitor), vec![p2_1.clone()]);
    // 4. Global (all other active popups conflict)
    assert_eq!(conflicts(PopupBehavior::Global), vec![p1_2, p2_1, p2_2]);
}

#[test]
fn test_compute_conflicts_never_returns_an_ancestor() {
    // Decision 7: opening a nested popup must never dismiss the popup it
    // lives inside, under any PopupBehavior — even Global, which otherwise
    // conflicts with everything else active.
    let screen = MonitorId::new("DP-1");
    let parent = PopupTarget::new(ModuleId::new(1), screen.clone());
    let child = PopupTarget::new(ModuleId::new(2), screen.clone());
    let unrelated = PopupTarget::new(ModuleId::new(3), screen);
    let active = vec![parent.clone(), unrelated.clone()];
    let ancestors = vec![parent.clone()];
    let opening = OpeningPopup::new(&child, &ancestors);
    let conflicts = |behavior| PopupExclusivity::compute_conflicts(&active, opening, behavior);

    for behavior in [
        PopupBehavior::PerModuleAndMonitor,
        PopupBehavior::PerModule,
        PopupBehavior::PerMonitor,
        PopupBehavior::Global,
    ] {
        let found = conflicts(behavior);
        assert!(
            !found.contains(&parent),
            "{behavior:?} must never return an ancestor"
        );
    }
    // Global still catches the unrelated, non-ancestor popup.
    assert_eq!(conflicts(PopupBehavior::Global), vec![unrelated]);
}

#[test]
fn test_ancestors_walks_the_chain_nearest_first() {
    let topmost = FloatingKind::Popup(PopupTarget::new(ModuleId::new(1), MonitorId::new("DP-1")));
    let middle = FloatingKind::Popup(PopupTarget::new(ModuleId::new(2), MonitorId::new("DP-1")));
    let innermost = FloatingKind::Popup(PopupTarget::new(ModuleId::new(3), MonitorId::new("DP-1")));

    // innermost's parent is middle, middle's parent is topmost, topmost has no parent.
    let links = [
        (innermost.clone(), Some(middle.clone())),
        (middle.clone(), Some(topmost.clone())),
    ];
    let parent_of = |child: &FloatingKind| {
        links
            .iter()
            .find(|(kind, _)| kind == child)
            .and_then(|(_, parent)| parent.clone())
    };

    let result = ancestors(&innermost, parent_of).expect("no cycle");
    assert_eq!(result, vec![middle, topmost]);
}

#[test]
fn test_ancestors_rejects_a_cycle() {
    let first = FloatingKind::Popup(PopupTarget::new(ModuleId::new(1), MonitorId::new("DP-1")));
    let second = FloatingKind::Popup(PopupTarget::new(ModuleId::new(2), MonitorId::new("DP-1")));

    // first's parent is second, second's parent is first: a cycle.
    let links = [
        (first.clone(), Some(second.clone())),
        (second, Some(first.clone())),
    ];
    let parent_of = |child: &FloatingKind| {
        links
            .iter()
            .find(|(kind, _)| kind == child)
            .and_then(|(_, parent)| parent.clone())
    };

    assert_eq!(ancestors(&first, parent_of), Err(PopupCycle));
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

/// Siblings at one depth keep the order of `links`, and every child of any node at the depth
/// above is found.
#[test]
fn test_descendants_deepest_first_keeps_link_order_within_a_depth() {
    let kind =
        |id| FloatingKind::Popup(PopupTarget::new(ModuleId::new(id), MonitorId::new("DP-1")));
    let (root, first, second, under_second, under_first) =
        (kind(1), kind(2), kind(3), kind(4), kind(5));

    let links = vec![
        (under_second.clone(), Some(second.clone())),
        (second.clone(), Some(root.clone())),
        (first.clone(), Some(root.clone())),
        (under_first.clone(), Some(first.clone())),
    ];

    let result = descendants_deepest_first(&root, &links);
    assert_eq!(result, vec![under_second, under_first, second, first]);
}
