//! The fixture's outputs and the delivery of events to any protocol object.

use super::Fake;
use crate::shared::wayland::adapters::wayland::state::WaylandState;
use crate::shared::wayland::adapters::wayland::types::WaylandOutputInfo;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::{Dispatch, Proxy};

impl Fake {
    /// A `wl_output` (version 4) bound as registry global `global_id`,
    /// recorded as `name` at scale 1.
    pub(crate) fn output_info(&self, global_id: u32, name: &str) -> WaylandOutputInfo {
        let registry = self
            .connection
            .display()
            .get_registry(&self.queue_handle, ());
        let output: WlOutput = registry.bind(global_id, 4, &self.queue_handle, ());
        WaylandOutputInfo {
            global_id,
            output,
            name: name.to_string(),
            scale: 1,
        }
    }

    /// Delivers the event of `addressed` to its object through `state`'s
    /// real dispatch handler.
    pub(crate) fn deliver<I>(&self, state: &mut WaylandState, addressed: (&I, I::Event))
    where
        I: Proxy,
        WaylandState: Dispatch<I, ()>,
    {
        let (proxy, event) = addressed;
        <WaylandState as Dispatch<I, ()>>::event(
            state,
            proxy,
            event,
            &(),
            &self.connection,
            &self.queue_handle,
        );
    }
}
