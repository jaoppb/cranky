//! `show_floating`: anchoring, nesting, conflicts and the offset a floating
//! surface is shown with, asserted on the state and on the positioner.

use crate::features::layout_engine::domain::{FloatingKind, PopupTarget, ShowFloating};
use crate::shared::config::domain::PopupEmbed;
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::primitives::{ModuleId, MonitorId, PopupOffset};
use crate::shared::wayland::adapters::wayland::floating_surface::{FloatingPayload, show_floating};
use crate::shared::wayland::adapters::wayland::state::WaylandState;
use crate::shared::wayland::adapters::wayland::tests::fixture::positioner::{
    SET_OFFSET, SET_SIZE, positioner_requests,
};
use crate::shared::wayland::adapters::wayland::tests::fixture::{Fake, popup_kind, solid};

fn show(fake: &Fake, state: &mut WaylandState, request: ShowFloating) {
    let payload = FloatingPayload {
        buffer: &request.buffer,
        logical_size: request.logical_size,
        offset: request.offset,
    };
    show_floating(
        state,
        &fake.queue_handle,
        request.kind,
        request.monitor_id,
        request.anchor_rect,
        &payload,
        request.parent,
    )
    .expect("shown");
}

/// A request to show `buffer` as `kind`, nested in `parent`, at the
/// buffer's own size, with no anchor rect and no offset of its own.
fn request(kind: FloatingKind, buffer: RenderBuffer, parent: Option<FloatingKind>) -> ShowFloating {
    ShowFloating {
        kind,
        monitor_id: None,
        anchor_rect: None,
        logical_size: *buffer.size(),
        buffer,
        offset: None,
        parent,
    }
}

fn popup_of(module: u32) -> FloatingKind {
    FloatingKind::Popup(PopupTarget::new(
        ModuleId::new(module),
        MonitorId::new("DP-1"),
    ))
}

/// A state with a bar on DP-1 and nothing shown.
fn with_bar(fake: &Fake) -> WaylandState {
    let mut state = fake.state(PopupEmbed::Composite);
    state.bars.push(fake.bar("DP-1", 1));
    fake.requests();
    state
}

#[test]
fn first_show_creates_the_surface_and_the_next_one_updates_it() {
    let fake = Fake::new();
    let mut state = with_bar(&fake);
    let base = solid(2, 1, [5, 5, 5, 255]);

    show(&fake, &mut state, request(popup_kind(), base.clone(), None));

    let floating = &state.floating_surfaces[&popup_kind()];
    let surface = floating.surface.clone();
    assert_eq!((&floating.base, floating.size), (&base, *base.size()));
    assert_eq!(floating.parent, None);
    // `[popup] offset` defaults to (0, 8).
    assert_eq!(positioner_requests(&fake, &state, SET_OFFSET), vec![[0, 8]]);

    let next = solid(2, 1, [2; 4]);
    show(&fake, &mut state, request(popup_kind(), next.clone(), None));
    assert_eq!(state.floating_surfaces.len(), 1);
    let floating = &state.floating_surfaces[&popup_kind()];
    assert_eq!((&floating.surface, &floating.base), (&surface, &next));
}

#[test]
fn own_offset_wins_and_a_panel_has_no_configured_one() {
    let fake = Fake::new();
    let mut state = with_bar(&fake);
    let mut popup = request(popup_kind(), solid(1, 1, [1; 4]), None);
    popup.offset = Some(PopupOffset::new(3, 4));
    show(&fake, &mut state, popup);
    assert_eq!(positioner_requests(&fake, &state, SET_OFFSET), vec![[3, 4]]);

    let panel = FloatingKind::Panel(PopupTarget::new(ModuleId::new(2), MonitorId::new("DP-1")));
    show(&fake, &mut state, request(panel, solid(1, 1, [1; 4]), None));
    assert_eq!(positioner_requests(&fake, &state, SET_OFFSET), vec![[0, 0]]);
}

#[test]
fn popup_on_a_monitor_without_a_bar_is_not_shown() {
    let fake = Fake::new();
    let mut state = fake.state(PopupEmbed::Composite);

    show(
        &fake,
        &mut state,
        request(popup_kind(), solid(1, 1, [1; 4]), None),
    );

    assert!(state.floating_surfaces.is_empty());
}

#[test]
fn nested_popup_keeps_its_parent_open_and_records_it() {
    let fake = Fake::new();
    let mut state = with_bar(&fake);
    show(
        &fake,
        &mut state,
        request(popup_kind(), solid(2, 1, [1; 4]), None),
    );

    let child = popup_of(9);
    show(
        &fake,
        &mut state,
        request(child.clone(), solid(1, 1, [2; 4]), Some(popup_kind())),
    );

    assert!(state.floating_surfaces.contains_key(&popup_kind()));
    assert_eq!(state.floating_surfaces[&child].parent, Some(popup_kind()));
}

#[test]
fn new_top_level_popup_closes_the_open_one() {
    let fake = Fake::new();
    let mut state = with_bar(&fake);
    show(
        &fake,
        &mut state,
        request(popup_kind(), solid(1, 1, [1; 4]), None),
    );

    show(
        &fake,
        &mut state,
        request(popup_of(2), solid(1, 1, [2; 4]), None),
    );

    assert!(!state.floating_surfaces.contains_key(&popup_kind()));
    assert!(state.floating_surfaces.contains_key(&popup_of(2)));
}

#[test]
fn nesting_cycle_is_dropped() {
    let fake = Fake::new();
    let mut state = with_bar(&fake);
    show(
        &fake,
        &mut state,
        request(popup_kind(), solid(1, 1, [1; 4]), None),
    );
    let child = popup_of(9);
    if let Some(parent) = state.floating_surfaces.get_mut(&popup_kind()) {
        parent.parent = Some(child.clone());
    }

    show(
        &fake,
        &mut state,
        request(child.clone(), solid(1, 1, [2; 4]), Some(popup_kind())),
    );

    assert!(!state.floating_surfaces.contains_key(&child));
    assert!(state.floating_surfaces.contains_key(&popup_kind()));
}

#[test]
fn positioner_gets_the_logical_size_and_the_surface_the_buffer_size() {
    let fake = Fake::new();
    let mut state = with_bar(&fake);
    let mut popup = request(popup_kind(), solid(4, 2, [1; 4]), None);
    popup.logical_size = Size::new(2, 1);

    show(&fake, &mut state, popup);

    assert_eq!(positioner_requests(&fake, &state, SET_SIZE), vec![[2, 1]]);
    let floating = &state.floating_surfaces[&popup_kind()];
    let shm_size = (floating.shm_buffer.width(), floating.shm_buffer.height());
    assert_eq!((floating.size, shm_size), (Size::new(4, 2), (4, 2)));
}
