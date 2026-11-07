//! Click-through surfaces, asserted on the requests that hit the wire.

use crate::features::layout_engine::domain::{FloatingKind, PopupTarget};
use crate::shared::config::domain::PopupEmbed;
use crate::shared::primitives::geometry::{Position, Size};
use crate::shared::primitives::{ModuleId, MonitorId, PopupOffset};
use crate::shared::wayland::adapters::wayland::command::SurfaceCommand;
use crate::shared::wayland::adapters::wayland::floating_anchor::{AnchorInfo, FloatingParentRole};
use crate::shared::wayland::adapters::wayland::floating_setup::{
    FloatingPlacement, create_new_floating,
};
use crate::shared::wayland::adapters::wayland::state::WaylandState;
use crate::shared::wayland::adapters::wayland::surface_handler::handle_cmd;
use crate::shared::wayland::adapters::wayland::tests::fixture::wire::{
    CREATE_REGION, REGION_ADD, REGION_DESTROY, Request, SET_INPUT_REGION, id,
};
use crate::shared::wayland::adapters::wayland::tests::fixture::{Fake, popup_kind, solid};
use crate::shared::wayland::surface_parent::SurfaceParent;

fn panel_kind() -> FloatingKind {
    FloatingKind::Panel(PopupTarget::new(ModuleId::new(1), MonitorId::new("DP-1")))
}

/// Opens `kind` over a bar on DP-1 and returns its surface's wire id.
fn open(fake: &Fake, state: &mut WaylandState, kind: &FloatingKind) -> u32 {
    let bar = fake.bar("DP-1", 1);
    let anchor = AnchorInfo {
        bar_scale: 1,
        parent_role: FloatingParentRole::Layer(bar.layer_surface.clone()),
        anchor_x: 0,
        anchor_y: 0,
        anchor_w: 1,
        anchor_h: 1,
        target_monitor_id: MonitorId::new("DP-1"),
    };
    state.bars.push(bar);
    let placement = FloatingPlacement {
        size: Size::new(10, 10),
        text_w: 10,
        text_h: 10,
        offset: PopupOffset::default(),
    };
    create_new_floating(
        state,
        &fake.qh,
        kind.clone(),
        &solid(10, 10, [1; 4]),
        &anchor,
        &placement,
        None,
    )
    .expect("create floating");
    id(&state.floating_surfaces[kind].surface)
}

fn region_for(requests: &[Request], surface: u32) -> Option<u32> {
    requests
        .iter()
        .find(|r| r.object == surface && r.opcode == SET_INPUT_REGION)
        .map(|r| r.arg)
}

fn assert_empty_region_set_then_destroyed(requests: &[Request], surface: u32) {
    let region = region_for(requests, surface).expect("tooltip surface got an input region");
    assert_ne!(region, 0, "a null region would restore the full input area");
    let at = |opcode, object| {
        requests
            .iter()
            .position(|r| r.opcode == opcode && r.object == object)
    };
    assert!(
        requests
            .iter()
            .any(|r| r.opcode == CREATE_REGION && r.arg == region),
        "region {region} was never created"
    );
    assert!(
        !requests
            .iter()
            .any(|r| r.object == region && r.opcode == REGION_ADD),
        "a region with rectangles is not empty"
    );
    let set = at(SET_INPUT_REGION, surface).expect("set");
    let destroy = at(REGION_DESTROY, region).expect("region destroyed");
    assert!(set < destroy, "region destroyed before it was applied");
}

#[test]
fn tooltip_surface_gets_an_empty_input_region() {
    let fake = Fake::new();
    let mut state = fake.state(PopupEmbed::Composite);
    let surface = open(&fake, &mut state, &FloatingKind::Tooltip);
    assert_empty_region_set_then_destroyed(&fake.requests(), surface);
}

#[test]
fn popup_and_panel_surfaces_stay_clickable() {
    for kind in [popup_kind(), panel_kind()] {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        let surface = open(&fake, &mut state, &kind);
        assert_eq!(region_for(&fake.requests(), surface), None, "{kind:?}");
    }
}

fn embed_under(fake: &Fake, state: &mut WaylandState, parent: &FloatingKind) -> u32 {
    open(fake, state, parent);
    let cmd = SurfaceCommand::new(
        ModuleId::new(2),
        Some(SurfaceParent::Floating(parent.clone())),
        MonitorId::new("DP-1"),
        Position::new(0, 0),
        solid(4, 4, [1; 4]),
    );
    handle_cmd(state, &fake.qh, &cmd).expect("embed child");
    let children = &state.floating_surfaces[parent].module_surfaces;
    id(&children[&ModuleId::new(2)].surface)
}

#[test]
fn subsurface_embedded_in_a_tooltip_is_click_through() {
    let fake = Fake::new();
    let mut state = fake.state(PopupEmbed::Subsurface);
    let child = embed_under(&fake, &mut state, &FloatingKind::Tooltip);
    assert_empty_region_set_then_destroyed(&fake.requests(), child);
}

#[test]
fn subsurface_embedded_in_a_popup_stays_clickable() {
    let fake = Fake::new();
    let mut state = fake.state(PopupEmbed::Subsurface);
    let child = embed_under(&fake, &mut state, &popup_kind());
    assert_eq!(region_for(&fake.requests(), child), None);
}
