//! `WaylandState::create_bar`: which outputs get a bar, the globals and memory
//! it needs, and the layer surface and scale a new bar gets.

use crate::shared::config::domain::PopupEmbed;
use crate::shared::primitives::MonitorId;
use crate::shared::primitives::geometry::Scale;
use crate::shared::wayland::adapters::wayland::state::WaylandState;
use crate::shared::wayland::adapters::wayland::tests::fixture::wire::id;
use crate::shared::wayland::adapters::wayland::tests::fixture::{Fake, lose_runtime_dir};
use crate::shared::wayland::adapters::wayland::types::ConfigureGate;
use crate::shared::wayland::ports::DisplayServerError;
use wayland_client::protocol::wl_output::WlOutput;

/// `zwlr_layer_surface_v1.{set_size,set_anchor,set_exclusive_zone,set_margin}`.
const SET_SIZE: u16 = 0;
const SET_ANCHOR: u16 = 1;
const SET_EXCLUSIVE_ZONE: u16 = 2;
const SET_MARGIN: u16 = 3;
/// `wl_surface.{commit,set_buffer_scale}`.
const COMMIT: u16 = 6;
const SET_BUFFER_SCALE: u16 = 8;

/// Adds `output`'s global id and name to `state` and returns its `wl_output`.
fn with_output(fake: &Fake, state: &mut WaylandState, output: (u32, &str)) -> WlOutput {
    let info = fake.output_info(output.0, output.1);
    let wl_output = info.output.clone();
    state.outputs.push(info);
    wl_output
}

fn create(
    fake: &Fake,
    state: &mut WaylandState,
    output: &WlOutput,
) -> Result<(), DisplayServerError> {
    state.create_bar(output, &fake.queue_handle)
}

/// Unbinds what a bar needs from a state.
type Unbind = fn(&mut WaylandState);

fn reason(result: Result<(), DisplayServerError>) -> String {
    match result {
        Err(DisplayServerError::ConnectionFailed { reason }) => reason,
        other => panic!("expected a connection failure, got {other:?}"),
    }
}

#[test]
fn output_the_state_does_not_know_fails() {
    let fake = Fake::new();
    let mut state = fake.state(PopupEmbed::Composite);
    let output = fake.output_info(7, "DP-1").output;
    assert_eq!(
        reason(create(&fake, &mut state, &output)),
        "Output not found"
    );
}

/// An unnamed output waits for its name, and an output keeps its one bar.
#[test]
fn unnamed_output_or_one_with_a_bar_gets_no_new_bar() {
    let fake = Fake::new();
    let mut state = fake.state(PopupEmbed::Composite);
    let unnamed = with_output(&fake, &mut state, (7, ""));
    create(&fake, &mut state, &unnamed).expect("unnamed output skipped");
    assert!(state.bars.is_empty());
    let named = with_output(&fake, &mut state, (8, "DP-1"));
    state.bars.push(fake.bar("DP-1", 1));
    create(&fake, &mut state, &named).expect("existing bar kept");
    assert_eq!(state.bars.len(), 1);
    assert_eq!(state.bars[0].width, 1);
}

/// A missing global fails with its name, checked in a fixed order, and a
/// buffer that cannot be allocated fails as an internal error.
#[test]
fn missing_globals_or_memory_fail_without_a_bar() {
    let fake = Fake::new();
    let unbinds: [(Unbind, &str); 4] = [
        (|state| state.compositor = None, "Compositor not bound"),
        (|state| state.layer_shell = None, "Layer shell not bound"),
        (|state| state.shm = None, "SHM not bound"),
        (
            |state| (state.compositor, state.shm) = (None, None),
            "Compositor not bound",
        ),
    ];
    for (unbind, expected) in unbinds {
        let mut state = fake.state(PopupEmbed::Composite);
        let output = with_output(&fake, &mut state, (7, "DP-1"));
        unbind(&mut state);
        assert_eq!(reason(create(&fake, &mut state, &output)), expected);
        assert!(state.bars.is_empty());
    }
    let mut state = fake.state(PopupEmbed::Composite);
    let output = with_output(&fake, &mut state, (7, "DP-1"));
    lose_runtime_dir(&mut state);
    let result = create(&fake, &mut state, &output);
    assert!(matches!(result, Err(DisplayServerError::Internal(_))));
    assert!(state.bars.is_empty());
}

/// A new bar is anchored to the top edge at the config height with its
/// exclusive zone, at the output's scale, which it publishes.
#[test]
fn new_bar_is_anchored_sized_and_publishes_its_scale() {
    let fake = Fake::new();
    let mut state = fake.state(PopupEmbed::Composite);
    let output = with_output(&fake, &mut state, (7, "DP-1"));
    state.outputs[0].scale = 2;
    fake.requests();
    create(&fake, &mut state, &output).expect("bar created");
    let bar = &state.bars[0];
    let geometry = (bar.output_name.as_str(), bar.width, bar.height, bar.scale);
    assert_eq!(geometry, ("DP-1", 1920, 30, 2));
    assert_eq!(bar.configure, ConfigureGate::Pending);
    let scales = state.hub.monitor_scales_rx().borrow().clone();
    assert_eq!(scales.get(&MonitorId::new("DP-1")), Some(&Scale::new(2.0)));
    let (surface, layer_surface) = (id(&bar.surface), id(&bar.layer_surface));
    let expected = vec![
        (surface, SET_BUFFER_SCALE, [2, 0, 0, 0]),
        (layer_surface, SET_ANCHOR, [13, 0, 0, 0]),
        (layer_surface, SET_SIZE, [0, 30, 0, 0]),
        (layer_surface, SET_MARGIN, [0; 4]),
        (layer_surface, SET_EXCLUSIVE_ZONE, [30, 0, 0, 0]),
        (surface, COMMIT, [0; 4]),
    ];
    assert_eq!(sent_to(&fake, [surface, layer_surface]), expected);
}

/// Each request flushed since the last call to one of `objects`, as
/// `(object, opcode, first argument words)`.
fn sent_to(fake: &Fake, objects: [u32; 2]) -> Vec<(u32, u16, [u32; 4])> {
    fake.requests()
        .iter()
        .filter(|request| objects.contains(&request.object))
        .map(|request| (request.object, request.opcode, request.args))
        .collect()
}
