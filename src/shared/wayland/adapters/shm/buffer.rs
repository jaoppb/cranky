//! A two-slot shared-memory buffer, so one slot can be drawn while the compositor still reads the
//! other.
use super::slot::{BufferSlot, BufferUserData};
use crate::shared::primitives::geometry::Size;
use cranky_unsafe::SharedMapping;
use std::io::Result;
use std::os::fd::AsFd;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::protocol::wl_shm_pool::WlShmPool;

/// Where shared-memory buffers come from: the compositor's `wl_shm` global and the directory
/// their backing files are created in.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ShmSource<'a> {
    shm: &'a WlShm,
    runtime_dir: &'a Path,
}

impl<'a> ShmSource<'a> {
    /// Allocates from the bound `shm` global, creating the backing files in `runtime_dir` (the
    /// session's `XDG_RUNTIME_DIR` in the app).
    pub(crate) const fn new(shm: &'a WlShm, runtime_dir: &'a Path) -> Self {
        Self { shm, runtime_dir }
    }
}

pub(crate) struct ShmBuffer {
    shm: SharedMapping,
    pool: WlShmPool,
    width: u32,
    height: u32,
    slots: [BufferSlot; 2],
    back_index: usize,
}

impl ShmBuffer {
    /// Allocates two `size` slots from `source`.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if creating or mapping the SHM file fails.
    pub(crate) fn new<S>(
        source: ShmSource<'_>,
        size: Size,
        queue_handle: &QueueHandle<S>,
    ) -> Result<Self>
    where
        S: wayland_client::Dispatch<wayland_client::protocol::wl_shm_pool::WlShmPool, ()>
            + wayland_client::Dispatch<wayland_client::protocol::wl_buffer::WlBuffer, BufferUserData>
            + 'static,
    {
        let (width, height) = (size.width(), size.height());
        let frame_size = frame_bytes(width, height);
        let total_size = frame_size.saturating_mul(2);
        let (shm, file) = SharedMapping::create(total_size, source.runtime_dir)?;

        let pool = source.shm.create_pool(
            file.as_fd(),
            i32::try_from(total_size).unwrap_or(i32::MAX),
            queue_handle,
            (),
        );

        let first = SlotGeometry::of_frame(width, height);
        let second = first.at_offset(i32::try_from(frame_size).unwrap_or_default());
        let slots = [
            create_slot(&pool, first, queue_handle),
            create_slot(&pool, second, queue_handle),
        ];

        Ok(Self {
            shm,
            pool,
            width,
            height,
            slots,
            back_index: 0,
        })
    }

    #[must_use]
    pub(crate) fn mmap_mut(&mut self) -> &mut [u8] {
        let frame_size = frame_bytes(self.width, self.height);
        let offset = self.back_index.saturating_mul(frame_size);
        if self
            .slots
            .get(self.back_index)
            .is_some_and(BufferSlot::is_busy)
        {
            log_busy_back_slot(self.back_index);
        }
        let end = offset.saturating_add(frame_size);
        self.shm
            .bytes_mut()
            .get_mut(offset..end)
            .unwrap_or_default()
    }

    #[must_use]
    pub(crate) fn current_buffer(&self) -> &wayland_client::protocol::wl_buffer::WlBuffer {
        self.slots
            .get(self.back_index)
            .map_or_else(|| self.slots[0].buffer(), BufferSlot::buffer)
    }

    #[must_use]
    pub(crate) const fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub(crate) const fn height(&self) -> u32 {
        self.height
    }

    pub(crate) fn swap_buffers(&mut self) {
        if let Some(slot) = self.slots.get(self.back_index) {
            slot.mark_busy();
        }
        self.back_index = 1usize.saturating_sub(self.back_index);
    }
}

impl Drop for ShmBuffer {
    fn drop(&mut self) {
        for slot in &self.slots {
            slot.buffer().destroy();
        }
        self.pool.destroy();
    }
}

/// The bytes one ARGB8888 frame of `width`×`height` takes.
fn frame_bytes(width: u32, height: u32) -> usize {
    usize::try_from(width.saturating_mul(height).saturating_mul(4)).unwrap_or_default()
}

/// Notes that `mmap_mut` hands out a back slot the compositor has not
/// released yet, which is drawn into anyway.
fn log_busy_back_slot(slot: usize) {
    tracing::debug!(
        slot,
        "Target back buffer is still marked busy by compositor; writing anyway"
    );
}

/// Where one slot's ARGB8888 frame sits in the pool, as `create_buffer` takes it.
#[derive(Clone, Copy)]
struct SlotGeometry {
    offset: i32,
    width: i32,
    height: i32,
    stride: i32,
}

impl SlotGeometry {
    /// A `width`×`height` frame at the start of the pool.
    fn of_frame(width: u32, height: u32) -> Self {
        Self {
            offset: 0,
            width: i32::try_from(width).unwrap_or_default(),
            height: i32::try_from(height).unwrap_or_default(),
            stride: i32::try_from(width.saturating_mul(4)).unwrap_or_default(),
        }
    }

    /// The same frame `offset` bytes into the pool.
    const fn at_offset(self, offset: i32) -> Self {
        Self { offset, ..self }
    }
}

/// A slot whose `wl_buffer` is `geometry` of `pool`, not busy.
fn create_slot<S>(
    pool: &WlShmPool,
    geometry: SlotGeometry,
    queue_handle: &QueueHandle<S>,
) -> BufferSlot
where
    S: wayland_client::Dispatch<wayland_client::protocol::wl_buffer::WlBuffer, BufferUserData>
        + 'static,
{
    let busy = Arc::new(AtomicBool::new(false));
    let buffer = pool.create_buffer(
        geometry.offset,
        geometry.width,
        geometry.height,
        geometry.stride,
        wayland_client::protocol::wl_shm::Format::Argb8888,
        queue_handle,
        BufferUserData::new(busy.clone()),
    );
    BufferSlot::new(buffer, busy)
}
