use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Debug)]
pub(crate) struct BufferUserData {
    busy: Arc<AtomicBool>,
}

impl BufferUserData {
    #[must_use]
    pub(crate) const fn new(busy: Arc<AtomicBool>) -> Self {
        Self { busy }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn is_busy(&self) -> bool {
        self.busy.load(Ordering::Acquire)
    }

    pub(crate) fn set_busy(&self, busy: bool) {
        self.busy.store(busy, Ordering::Release);
    }
}

pub(in crate::shared::wayland::adapters) struct BufferSlot {
    buffer: wayland_client::protocol::wl_buffer::WlBuffer,
    busy: Arc<AtomicBool>,
}

impl BufferSlot {
    #[must_use]
    pub(crate) const fn new(
        buffer: wayland_client::protocol::wl_buffer::WlBuffer,
        busy: Arc<AtomicBool>,
    ) -> Self {
        Self { buffer, busy }
    }

    #[must_use]
    pub(crate) const fn buffer(&self) -> &wayland_client::protocol::wl_buffer::WlBuffer {
        &self.buffer
    }

    #[must_use]
    pub(crate) fn is_busy(&self) -> bool {
        self.busy.load(Ordering::Acquire)
    }

    pub(crate) fn set_busy(&self, busy: bool) {
        self.busy.store(busy, Ordering::Release);
    }
}
