use super::lifecycle::{create_test_event_loop_and_channel, outcome_with, popup_text};
use crate::features::layout_engine::domain::{
    DisplayCommand, FloatingKind, PopupTarget, ShowFloating,
};
use crate::features::module_runtime::domain::PopupRenderLayout;
use crate::shared::primitives::geometry::{Position, Rect, Size};
use crate::shared::primitives::{MonitorId, PopupOffset};

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
    let popup_content = popup_text(Size::new(30, 12));

    let first_anchor = Rect::new(Position::new(0, 0), Size::new(50, 20));
    let popup_layout1 = PopupRenderLayout::new(first_anchor, popup_content.clone(), None);
    event_loop.dispatch_render_outcome(&mon, &outcome_with(Some(popup_layout1)));
    display_rx
        .try_recv()
        .expect("First dispatch should have sent ShowFloatingSurface");

    // Second tick: same content, but the anchor moved (e.g. a sibling
    // module's width changed this module's x-offset).
    let second_anchor = Rect::new(Position::new(40, 0), Size::new(50, 20));
    assert_ne!(first_anchor, second_anchor);
    let popup_layout2 = PopupRenderLayout::new(second_anchor, popup_content, None);
    event_loop.dispatch_render_outcome(&mon, &outcome_with(Some(popup_layout2)));

    let cmd2 = display_rx
        .try_recv()
        .expect("Anchor-only move must still dispatch a fresh ShowFloatingSurface");
    let DisplayCommand::ShowFloatingSurface(ShowFloating {
        kind, anchor_rect, ..
    }) = cmd2
    else {
        panic!("Expected ShowFloatingSurface, got {cmd2:?}");
    };
    assert_eq!(kind, FloatingKind::Popup(PopupTarget::new(id, mon)));
    assert_eq!(anchor_rect, Some(second_anchor));
}

fn anchor() -> Rect {
    Rect::new(Position::new(0, 0), Size::new(50, 20))
}

/// The same popup at the same anchor is sent once; new content at that anchor is sent again.
#[test]
fn an_unchanged_popup_is_sent_once_and_new_content_again() {
    let (mut event_loop, _id, display_rx) = create_test_event_loop_and_channel();
    let mon = MonitorId::new("DP-1");
    let small = PopupRenderLayout::new(anchor(), popup_text(Size::new(30, 12)), None);
    event_loop.dispatch_render_outcome(&mon, &outcome_with(Some(small.clone())));
    event_loop.dispatch_render_outcome(&mon, &outcome_with(Some(small)));
    assert_eq!(display_rx.try_iter().count(), 1);

    let wide = PopupRenderLayout::new(anchor(), popup_text(Size::new(60, 12)), None);
    event_loop.dispatch_render_outcome(&mon, &outcome_with(Some(wide)));
    let shown = display_rx.try_recv();
    assert!(
        matches!(&shown, Ok(DisplayCommand::ShowFloatingSurface(show)) if show.logical_size == Size::new(60, 12)),
        "expected the wider popup shown, got {shown:?}"
    );
}

/// With nothing shown, an outcome with nothing open sends nothing; the popup's offset goes out
/// with it.
#[test]
fn closing_what_was_never_shown_sends_nothing_and_an_offset_goes_out_with_the_popup() {
    let (mut event_loop, _id, display_rx) = create_test_event_loop_and_channel();
    let mon = MonitorId::new("DP-1");
    event_loop.dispatch_render_outcome(&mon, &outcome_with(None));
    assert!(display_rx.try_recv().is_err());

    let offset = PopupOffset::new(3, 4);
    let popup = PopupRenderLayout::new(anchor(), popup_text(Size::new(30, 12)), Some(offset));
    event_loop.dispatch_render_outcome(&mon, &outcome_with(Some(popup)));
    let shown = display_rx.try_recv();
    assert!(
        matches!(&shown, Ok(DisplayCommand::ShowFloatingSurface(show)) if show.offset == Some(offset)),
        "expected the popup shown with its offset, got {shown:?}"
    );
}

/// A popup with no area paints nothing and is not shown, but it is still remembered as shown:
/// closing it sends a hide.
#[test]
fn a_popup_with_no_area_is_not_shown_yet_closing_it_sends_a_hide() {
    let (mut event_loop, id, display_rx) = create_test_event_loop_and_channel();
    let mon = MonitorId::new("DP-1");
    let empty = PopupRenderLayout::new(anchor(), popup_text(Size::new(0, 12)), None);
    event_loop.dispatch_render_outcome(&mon, &outcome_with(Some(empty)));
    assert!(display_rx.try_recv().is_err());

    event_loop.dispatch_render_outcome(&mon, &outcome_with(None));
    let target = PopupTarget::new(id, mon);
    assert_eq!(
        display_rx.try_iter().collect::<Vec<_>>(),
        [DisplayCommand::HideFloatingSurface {
            kind: FloatingKind::Popup(target)
        }]
    );
}
