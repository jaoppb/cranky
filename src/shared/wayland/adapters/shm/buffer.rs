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
        let frame_size =
            usize::try_from(width.saturating_mul(height).saturating_mul(4)).unwrap_or_default();
        let total_size = frame_size.saturating_mul(2);
        let (shm, file) = SharedMapping::create(total_size, source.runtime_dir)?;

        let pool = source.shm.create_pool(
            file.as_fd(),
            i32::try_from(total_size).unwrap_or(i32::MAX),
            queue_handle,
            (),
        );

        let width_i32 = i32::try_from(width).unwrap_or_default();
        let height_i32 = i32::try_from(height).unwrap_or_default();
        let stride_i32 = i32::try_from(width.saturating_mul(4)).unwrap_or_default();

        let busy_0 = Arc::new(AtomicBool::new(false));
        let user_data_0 = BufferUserData::new(busy_0.clone());
        let buffer_0 = pool.create_buffer(
            0,
            width_i32,
            height_i32,
            stride_i32,
            wayland_client::protocol::wl_shm::Format::Argb8888,
            queue_handle,
            user_data_0,
        );

        let busy_1 = Arc::new(AtomicBool::new(false));
        let user_data_1 = BufferUserData::new(busy_1.clone());
        let buffer_1 = pool.create_buffer(
            i32::try_from(frame_size).unwrap_or_default(),
            width_i32,
            height_i32,
            stride_i32,
            wayland_client::protocol::wl_shm::Format::Argb8888,
            queue_handle,
            user_data_1,
        );

        let slots = [
            BufferSlot::new(buffer_0, busy_0),
            BufferSlot::new(buffer_1, busy_1),
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
        let frame_size = usize::try_from(self.width.saturating_mul(self.height).saturating_mul(4))
            .unwrap_or_default();
        let offset = self.back_index.saturating_mul(frame_size);
        if let Some(slot) = self.slots.get(self.back_index)
            && slot.is_busy()
        {
            tracing::debug!(
                slot = self.back_index,
                "Target back buffer is still marked busy by compositor; writing anyway"
            );
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
