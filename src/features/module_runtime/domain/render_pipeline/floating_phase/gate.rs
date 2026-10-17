use super::FloatingLayouts;
use crate::features::layout_engine::domain::RenderNode;
use crate::shared::primitives::{ChildSizesMap, MonitorId};
use std::collections::HashMap;

/// How many consecutive evaluations may hold a floating surface back before
/// it is presented anyway. A count, not a timer, so behavior does not depend
/// on machine load.
pub(super) const MAX_HOLD_ROUNDS: u8 = 4;

/// True while any module embedded in `node` has not yet laid itself out at
/// the width its host box resolved to.
///
/// A module with no report at all is unsettled (its host is still a
/// placeholder). A zero-width host has nothing to settle at.
fn is_unsettled(node: &RenderNode, sizes: Option<&ChildSizesMap>) -> bool {
    node.collect_module_layouts().iter().any(|layout| {
        let host_width = layout.bounds().width();
        let report = sizes
            .and_then(|s| s.report_by_name_or_key(layout.key().name(), layout.key().instance_id()));
        report.is_none_or(|r| host_width > 0 && r.settled_width() != Some(host_width))
    })
}

/// Marks floating layouts whose embedded children have not converged on the
/// host box as held, so the surface keeps what it last showed (or stays
/// closed) instead of presenting a wrong-width frame.
///
/// After `MAX_HOLD_ROUNDS` consecutive holds the latest layout is presented
/// with a warning: a CSS mistake that never converges must not make the
/// popup silently vanish.
pub(in super::super) fn apply_gate(
    floating: FloatingLayouts,
    sizes: Option<&ChildSizesMap>,
    rounds: &mut HashMap<MonitorId, u8>,
    monitor_id: &MonitorId,
) -> FloatingLayouts {
    let popup = floating
        .popup()
        .is_some_and(|p| is_unsettled(p.node(), sizes));
    let panel = floating.panel().is_some_and(|n| is_unsettled(n, sizes));
    let tooltip = floating.tooltip().is_some_and(|n| is_unsettled(n, sizes));
    if !(popup || panel || tooltip) {
        rounds.remove(monitor_id);
        return floating;
    }
    let held_rounds = rounds.entry(monitor_id.clone()).or_insert(0);
    if *held_rounds >= MAX_HOLD_ROUNDS {
        tracing::warn!(
            monitor = %monitor_id,
            "Embedded module did not settle on its host width; presenting the latest layout"
        );
        return floating;
    }
    *held_rounds = held_rounds.saturating_add(1);
    floating.with_held(popup, panel, tooltip)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::layout_engine::domain::{NodePath, SurfaceSpace};
    use crate::features::module_runtime::domain::PopupRenderLayout;
    use crate::features::styling::domain::ComputedStyle;
    use crate::shared::primitives::geometry::{Position, Rect, Size};
    use crate::shared::primitives::{ChildReport, ModuleKey, ModuleName, ModuleOptions};

    fn calendar_key() -> ModuleKey {
        ModuleKey::from_name(ModuleName::new("calendar"))
    }

    fn host(width: u32) -> RenderNode {
        RenderNode::Module {
            path: NodePath::root_in(SurfaceSpace::Popup),
            node_key: None,
            rect: Rect::new(Position::new(0, 0), Size::new(width, 190)),
            key: calendar_key(),
            options: ModuleOptions::default(),
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        }
    }

    fn popup_of(node: RenderNode) -> FloatingLayouts {
        let anchor = Rect::new(Position::new(0, 0), Size::new(40, 20));
        FloatingLayouts::new(Some(PopupRenderLayout::new(anchor, node, None)), None, None)
    }

    fn sizes(settled: Option<u32>) -> ChildSizesMap {
        let mut map = ChildSizesMap::new();
        map.insert_report(
            calendar_key(),
            ChildReport::new(Size::new(240, 190), settled),
        );
        map
    }

    fn run(
        floating: FloatingLayouts,
        sizes: Option<&ChildSizesMap>,
        rounds: &mut HashMap<MonitorId, u8>,
    ) -> bool {
        apply_gate(floating, sizes, rounds, &MonitorId::new("DP-1")).popup_held()
    }

    #[test]
    fn holds_while_the_child_has_not_reported() {
        let mut rounds = HashMap::new();
        assert!(run(popup_of(host(280)), None, &mut rounds));
    }

    #[test]
    fn holds_until_the_child_settled_at_the_host_width() {
        let mut rounds = HashMap::new();
        // Reported intrinsic 240 but still laid out unpinned / at 240.
        assert!(run(popup_of(host(280)), Some(&sizes(None)), &mut rounds));
        assert!(run(
            popup_of(host(280)),
            Some(&sizes(Some(240))),
            &mut rounds
        ));
        assert!(!run(
            popup_of(host(280)),
            Some(&sizes(Some(280))),
            &mut rounds
        ));
    }

    #[test]
    fn settling_resets_the_round_counter() {
        let mut rounds = HashMap::new();
        assert!(run(popup_of(host(280)), None, &mut rounds));
        assert!(!run(
            popup_of(host(280)),
            Some(&sizes(Some(280))),
            &mut rounds
        ));
        assert!(rounds.is_empty());
    }

    #[test]
    fn presents_the_latest_layout_after_the_round_cap() {
        let mut rounds = HashMap::new();
        for _ in 0..MAX_HOLD_ROUNDS {
            assert!(run(
                popup_of(host(280)),
                Some(&sizes(Some(240))),
                &mut rounds
            ));
        }
        // Never converged: present anyway rather than vanish silently.
        assert!(!run(
            popup_of(host(280)),
            Some(&sizes(Some(240))),
            &mut rounds
        ));
    }

    #[test]
    fn floating_content_without_embedded_modules_is_never_held() {
        let mut rounds = HashMap::new();
        let mut plain = host(0);
        if let RenderNode::Module { .. } = plain {
            plain = RenderNode::Rect {
                path: NodePath::root_in(SurfaceSpace::Popup),
                node_key: None,
                rect: Rect::new(Position::new(0, 0), Size::new(10, 10)),
                style: ComputedStyle::default(),
                on_click: None,
                on_hover: None,
                tooltip: None,
                popup: None,
                panel: None,
            };
        }
        assert!(!run(popup_of(plain), None, &mut rounds));
    }
}
