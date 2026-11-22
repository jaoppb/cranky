use super::state::WaylandState;
use crate::shared::wayland::adapters::shm::BufferUserData;
use wayland_client::protocol::{
    wl_buffer::{self, WlBuffer},
    wl_compositor::{self, WlCompositor},
    wl_shm::{self, WlShm},
    wl_shm_pool::{self, WlShmPool},
    wl_subcompositor::{self, WlSubcompositor},
    wl_subsurface::{self, WlSubsurface},
    wl_surface::{self, WlSurface},
};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::{self, ZwlrLayerShellV1};

impl Dispatch<WlCompositor, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlCompositor,
        _: wl_compositor::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlShm, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlShm,
        _: wl_shm::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrLayerShellV1, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &ZwlrLayerShellV1,
        _: zwlr_layer_shell_v1::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlSubcompositor, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlSubcompositor,
        _: wl_subcompositor::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlSubsurface, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlSubsurface,
        _: wl_subsurface::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlSurface, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlSurface,
        _: wl_surface::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlBuffer, BufferUserData> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlBuffer,
        event: wl_buffer::Event,
        data: &BufferUserData,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let wl_buffer::Event::Release = event else {
            return;
        };
        data.mark_released();
        tracing::trace!("WlBuffer released by compositor");
    }
}

impl Dispatch<WlShmPool, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlShmPool,
        _: wl_shm_pool::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

#[cfg(test)]
mod tests {
    use crate::shared::config::domain::PopupEmbed;
    use crate::shared::primitives::geometry::Size;
    use crate::shared::wayland::adapters::shm::BufferUserData;
    use crate::shared::wayland::adapters::wayland::state::WaylandState;
    use crate::shared::wayland::adapters::wayland::tests::fixture::Fake;
    use wayland_client::protocol::wl_buffer::{self, WlBuffer};
    use wayland_client::{Dispatch, Proxy};

    fn user_data(buffer: &WlBuffer) -> BufferUserData {
        buffer
            .data::<BufferUserData>()
            .expect("buffer user data")
            .clone()
    }

    /// The release event frees the slot the compositor held.
    #[test]
    fn buffer_release_frees_its_slot() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        let mut shm = fake.shm_buffer(Size::new(1, 1));
        let held = shm.current_buffer().clone();
        shm.swap_buffers();
        let data = user_data(&held);
        assert!(data.is_busy());
        <WaylandState as Dispatch<WlBuffer, BufferUserData>>::event(
            &mut state,
            &held,
            wl_buffer::Event::Release,
            &data,
            fake.connection(),
            &fake.queue_handle,
        );
        assert!(!data.is_busy());
    }

    /// A back slot the compositor still holds is drawn into anyway, whole.
    #[test]
    fn busy_back_slot_is_still_drawn_into() {
        let fake = Fake::new();
        let mut shm = fake.shm_buffer(Size::new(2, 1));
        shm.swap_buffers();
        shm.swap_buffers();
        assert!(user_data(shm.current_buffer()).is_busy());
        let frame = shm.mmap_mut();
        assert_eq!(frame.len(), 8);
        frame.fill(7);
        assert_eq!(shm.mmap_mut(), &[7; 8]);
    }

    /// Both slots come from one pool two frames long: the first at offset 0,
    /// the second right after it, each a whole frame.
    #[test]
    fn buffer_slots_split_one_pool() {
        let fake = Fake::new();
        fake.requests();
        let _shm = fake.shm_buffer(Size::new(3, 2));
        let requests = fake.requests();
        let [pool, first, second] = requests.as_slice() else {
            panic!("expected create_pool and two create_buffer: {requests:?}");
        };
        assert_eq!(pool.args[1], 48);
        for (slot, offset) in [(first, 0), (second, 24)] {
            assert_eq!(slot.object, pool.arg);
            assert_eq!(slot.args[1..], [offset, 3, 2]);
        }
    }
}
