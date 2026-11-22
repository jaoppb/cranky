use super::{FloatingHold, FloatingLayouts, PopupRenderLayout};
use crate::features::layout_engine::domain::RenderNode;
use crate::shared::primitives::{ChildSizesMap, MonitorId};
use std::collections::hash_map::Entry;

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
        let host_width = layout.bounds().rect().width();
        let report = sizes.and_then(|child_sizes| {
            child_sizes.report_by_name_or_key(layout.key().name(), layout.key().instance_id())
        });
        report.is_none_or(|size_report| {
            host_width > 0 && size_report.settled_width() != Some(host_width)
        })
    })
}

/// Holds a floating kind whose content has an embedded module that has not settled yet.
fn hold_for(node: Option<&RenderNode>, sizes: Option<&ChildSizesMap>) -> FloatingHold {
    if node.is_some_and(|content| is_unsettled(content, sizes)) {
        FloatingHold::Held
    } else {
        FloatingHold::Present
    }
}

/// Marks floating layouts whose embedded children have not converged on the
/// host box as held, so the surface keeps what it last showed (or stays
/// closed) instead of presenting a wrong-width frame.
///
/// After `MAX_HOLD_ROUNDS` consecutive holds the latest layout is presented
/// with a warning: a CSS mistake that never converges must not make the
/// popup silently vanish.
///
/// `rounds` is the monitor's entry in the count of consecutive holds.
pub(in super::super) fn apply_gate(
    floating: FloatingLayouts,
    sizes: Option<&ChildSizesMap>,
    rounds: Entry<'_, MonitorId, u8>,
) -> FloatingLayouts {
    let popup = hold_for(floating.popup().map(PopupRenderLayout::node), sizes);
    let panel = hold_for(floating.panel(), sizes);
    let tooltip = hold_for(floating.tooltip(), sizes);
    let held = [popup, panel, tooltip];
    if !held.contains(&FloatingHold::Held) {
        if let Entry::Occupied(counted) = rounds {
            counted.remove();
        }
        return floating;
    }
    if let Entry::Occupied(counted) = &rounds
        && *counted.get() >= MAX_HOLD_ROUNDS
    {
        tracing::warn!(
            monitor = %rounds.key(),
            "Embedded module did not settle on its host width; presenting the latest layout"
        );
        return floating;
    }
    let held_rounds = rounds.or_insert(0);
    *held_rounds = held_rounds.saturating_add(1);
    floating.with_held(held)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::layout_engine::domain::{NodePath, SurfaceSpace};
    use crate::features::module_runtime::domain::PopupRenderLayout;
    use crate::features::styling::domain::ComputedStyle;
    use crate::shared::primitives::geometry::{Position, Rect, Size};
    use crate::shared::primitives::{ChildReport, ModuleKey, ModuleName, ModuleOptions};
    use std::collections::HashMap;

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
        apply_gate(floating, sizes, rounds.entry(MonitorId::new("DP-1"))).popup_hold()
            == FloatingHold::Held
    }

    /// Which of popup, panel and tooltip `floating` holds after the gate.
    fn held_kinds(floating: FloatingLayouts) -> (FloatingHold, FloatingHold, FloatingHold) {
        let gated = apply_gate(floating, None, HashMap::new().entry(MonitorId::new("DP-1")));
        (gated.popup_hold(), gated.panel_hold(), gated.tooltip_hold())
    }

    #[test]
    fn holds_only_the_kind_whose_content_is_unsettled() {
        use FloatingHold::{Held, Present};
        assert_eq!(held_kinds(popup_of(host(280))), (Held, Present, Present));
        assert_eq!(
            held_kinds(FloatingLayouts::new(None, Some(host(280)), None)),
            (Present, Held, Present)
        );
        assert_eq!(
            held_kinds(FloatingLayouts::new(None, None, Some(host(280)))),
            (Present, Present, Held)
        );
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
