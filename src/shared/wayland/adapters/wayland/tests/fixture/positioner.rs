//! Reads back what the client sent to the positioners it created.

use super::Fake;
use super::wire::id;
use crate::shared::wayland::adapters::wayland::state::WaylandState;

/// `xdg_wm_base.create_positioner`.
const CREATE_POSITIONER: u16 = 1;
/// `xdg_positioner.set_size`.
pub(in crate::shared::wayland::adapters::wayland::tests) const SET_SIZE: u16 = 1;
/// `xdg_positioner.set_offset`.
pub(in crate::shared::wayland::adapters::wayland::tests) const SET_OFFSET: u16 = 6;

/// The first two arguments of every `opcode` request sent to a positioner
/// created since the last call.
pub(in crate::shared::wayland::adapters::wayland::tests) fn positioner_requests(
    fake: &Fake,
    state: &WaylandState,
    opcode: u16,
) -> Vec<[u32; 2]> {
    let wm_base = state.xdg_wm_base.as_ref().map(id).expect("xdg_wm_base");
    let requests = fake.requests();
    let positioners: Vec<u32> = requests
        .iter()
        .filter(|request| request.object == wm_base && request.opcode == CREATE_POSITIONER)
        .map(|request| request.arg)
        .collect();
    requests
        .iter()
        .filter(|request| positioners.contains(&request.object) && request.opcode == opcode)
        .map(|request| [request.args[0], request.args[1]])
        .collect()
}
