use super::slot::{BufferSlot, BufferUserData};
use cranky_unsafe::SharedMapping;
use std::io::Result;
use std::os::fd::AsFd;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::protocol::wl_shm_pool::WlShmPool;

pub struct ShmBuffer {
    shm: SharedMapping,
    pool: WlShmPool,
    width: u32,
    height: u32,
    slots: [BufferSlot; 2],
    back_index: usize,
}

impl ShmBuffer {
    /// # Errors
    ///
    /// Returns an I/O error if creating or mapping the SHM file fails.
    pub fn new<S>(
        shm_proxy: &WlShm,
        width: u32,
        height: u32,
        queue_handle: &QueueHandle<S>,
        xdg_runtime_dir: &std::path::Path,
    ) -> Result<Self>
    where
        S: wayland_client::Dispatch<wayland_client::protocol::wl_shm_pool::WlShmPool, ()>
            + wayland_client::Dispatch<wayland_client::protocol::wl_buffer::WlBuffer, BufferUserData>
            + 'static,
    {
        let frame_size =
            usize::try_from(width.saturating_mul(height).saturating_mul(4)).unwrap_or_default();
        let total_size = frame_size.saturating_mul(2);
        let (shm, file) = SharedMapping::create(total_size, xdg_runtime_dir)?;

        let pool = shm_proxy.create_pool(
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
    pub fn mmap_mut(&mut self) -> &mut [u8] {
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
    pub fn current_buffer(&self) -> &wayland_client::protocol::wl_buffer::WlBuffer {
        self.slots
            .get(self.back_index)
            .map_or_else(|| self.slots[0].buffer(), BufferSlot::buffer)
    }

    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    pub fn swap_buffers(&mut self) {
        if let Some(slot) = self.slots.get(self.back_index) {
            slot.set_busy(true);
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
