//! One `wl_buffer` of a shared-memory pool, with the busy flag its release event clears.
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

    /// Frees the buffer for drawing again: the compositor released it.
    pub(crate) fn mark_released(&self) {
        self.busy.store(false, Ordering::Release);
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

    /// Holds the buffer until the compositor releases it: it was just handed over for display.
    pub(crate) fn mark_busy(&self) {
        self.busy.store(true, Ordering::Release);
    }
}
