use super::lifecycle::{create_test_event_loop_and_channel, make_test_rect_node};
use crate::features::layout_engine::domain::{DisplayCommand, FloatingKind};
use crate::features::module_runtime::domain::{FloatingLayouts, PopupRenderLayout};
use crate::features::styling::domain::ComputedStyle;
use crate::shared::primitives::MonitorId;
use crate::shared::primitives::geometry::{Position, Rect, Scale, Size};

/// Regression test: a popup's content can stay byte-identical across two
/// render ticks while its anchor moves (a sibling module's width shifting
/// this module's x-offset on the bar). The dedup cache must key on
/// (`anchor_rect`, node), not just the node, or the second dispatch gets
/// skipped as "unchanged" and the popup stays visually stuck at its old
/// position even though Wayland was never told to reposition it.
#[test]
fn test_event_loop_popup_dispatch_refreshes_on_anchor_move_alone() {
    let (mut event_loop, id, display_rx) = create_test_event_loop_and_channel();
    let mon = MonitorId::new("DP-1");

    // Identical content on both ticks.
    let popup_content = crate::features::layout_engine::domain::RenderNode::Text {
        path: crate::features::layout_engine::domain::NodePath::root_in(
            crate::features::layout_engine::domain::SurfaceSpace::Bar,
        ),
        node_key: None,
        rect: Rect::new(Position::new(0, 0), Size::new(30, 12)),
        text: crate::features::vdom::domain::TextContent::new("popup".to_string()),
        style: ComputedStyle::default(),
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    };

    let first_anchor = Rect::new(Position::new(0, 0), Size::new(50, 20));
    let popup_layout1 = PopupRenderLayout::new(first_anchor, popup_content.clone(), None);
    let outcome1 = crate::features::module_runtime::domain::RenderOutcome::new(
        None,
        vec![],
        make_test_rect_node(),
        None,
        FloatingLayouts::new(Some(popup_layout1), None, None),
    );
    event_loop.dispatch_render_outcome(&mon, &outcome1, Scale::new(1.0));
    display_rx
        .try_recv()
        .expect("First dispatch should have sent ShowFloatingSurface");

    // Second tick: same content, but the anchor moved (e.g. a sibling
    // module's width changed this module's x-offset).
    let second_anchor = Rect::new(Position::new(40, 0), Size::new(50, 20));
    assert_ne!(first_anchor, second_anchor);
    let popup_layout2 = PopupRenderLayout::new(second_anchor, popup_content, None);
    let outcome2 = crate::features::module_runtime::domain::RenderOutcome::new(
        None,
        vec![],
        make_test_rect_node(),
        None,
        FloatingLayouts::new(Some(popup_layout2), None, None),
    );
    event_loop.dispatch_render_outcome(&mon, &outcome2, Scale::new(1.0));

    let cmd2 = display_rx
        .try_recv()
        .expect("Anchor-only move must still dispatch a fresh ShowFloatingSurface");
    match cmd2 {
        DisplayCommand::ShowFloatingSurface(
            crate::features::layout_engine::domain::ShowFloating {
                kind, anchor_rect, ..
            },
        ) => {
            assert_eq!(
                kind,
                FloatingKind::Popup(crate::features::layout_engine::domain::PopupTarget::new(
                    id, mon,
                ))
            );
            assert_eq!(anchor_rect, Some(second_anchor));
        }
        _ => panic!("Expected ShowFloatingSurface, got {cmd2:?}"),
    }
}
