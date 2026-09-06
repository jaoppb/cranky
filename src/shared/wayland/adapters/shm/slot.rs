use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Debug)]
pub struct BufferUserData {
    busy: Arc<AtomicBool>,
}

impl BufferUserData {
    #[must_use]
    pub const fn new(busy: Arc<AtomicBool>) -> Self {
        Self { busy }
    }

    #[must_use]
    pub fn is_busy(&self) -> bool {
        self.busy.load(Ordering::Acquire)
    }

    pub fn set_busy(&self, busy: bool) {
        self.busy.store(busy, Ordering::Release);
    }
}

pub struct BufferSlot {
    buffer: wayland_client::protocol::wl_buffer::WlBuffer,
    busy: Arc<AtomicBool>,
}

impl BufferSlot {
    #[must_use]
    pub const fn new(
        buffer: wayland_client::protocol::wl_buffer::WlBuffer,
        busy: Arc<AtomicBool>,
    ) -> Self {
        Self { buffer, busy }
    }

    #[must_use]
    pub const fn buffer(&self) -> &wayland_client::protocol::wl_buffer::WlBuffer {
        &self.buffer
    }

    #[must_use]
    pub fn is_busy(&self) -> bool {
        self.busy.load(Ordering::Acquire)
    }

    pub fn set_busy(&self, busy: bool) {
        self.busy.store(busy, Ordering::Release);
    }
}
