#![allow(unsafe_code)]

use memmap2::MmapMut;
use std::fs::File;
use std::io::Result;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::BorrowedFd;
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::protocol::wl_shm_pool::WlShmPool;

pub struct MmappedShm {
    mmap: MmapMut,
}

impl MmappedShm {
    #[cfg(test)]
    /// # Errors
    ///
    /// Returns an I/O error if creating or memory-mapping the SHM file fails.
    pub fn new(size: usize, xdg_runtime_dir: &std::path::Path) -> Result<Self> {
        let file = create_shm_file(size, xdg_runtime_dir)?;
        let mmap = safe_mmap_file(&file)?;
        Ok(Self { mmap })
    }

    #[must_use]
    pub fn mmap_mut(&mut self) -> &mut [u8] {
        &mut self.mmap
    }

    #[cfg(test)]
    #[must_use]
    pub fn size(&self) -> usize {
        self.mmap.len()
    }
}

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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
    pub const fn new(buffer: wayland_client::protocol::wl_buffer::WlBuffer, busy: Arc<AtomicBool>) -> Self {
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

pub struct ShmBuffer {
    shm: MmappedShm,
    pool: WlShmPool,
    width: u32,
    height: u32,
    slots: [BufferSlot; 2],
    back_index: usize,
}

fn create_shm_file(size: usize, xdg_runtime_dir: &std::path::Path) -> Result<File> {
    let mut path = xdg_runtime_dir.to_path_buf();

    let id = uuid::Uuid::new_v4();
    path.push(format!("cranky-shm-{id}"));

    let file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)?;

    // Immediately unlink the file so it's only accessible via the FD
    let _ = std::fs::remove_file(&path);
    let len = u64::try_from(size).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    file.set_len(len)?;
    Ok(file)
}

// Safe wrapper around unsafe mmap creation
fn safe_mmap_file(file: &File) -> Result<MmapMut> {
    unsafe { MmapMut::map_mut(file) }
}

// Safe wrapper around unsafe BorrowedFd creation for file descriptors
fn safe_borrowed_fd_from_file(file: &File) -> BorrowedFd<'_> {
    unsafe { BorrowedFd::borrow_raw(file.as_raw_fd()) }
}

impl ShmBuffer {
    /// # Errors
    ///
    /// Returns an I/O error if creating or mapping the SHM file fails.
    pub fn new<S>(
        shm_proxy: &WlShm,
        width: u32,
        height: u32,
        qh: &QueueHandle<S>,
        xdg_runtime_dir: &std::path::Path,
    ) -> Result<Self>
    where
        S: wayland_client::Dispatch<wayland_client::protocol::wl_shm_pool::WlShmPool, ()>
            + wayland_client::Dispatch<wayland_client::protocol::wl_buffer::WlBuffer, BufferUserData>
            + 'static,
    {
        let frame_size = usize::try_from(width.saturating_mul(height).saturating_mul(4)).unwrap_or_default();
        let total_size = frame_size.saturating_mul(2);
        let file = create_shm_file(total_size, xdg_runtime_dir)?;

        let mmap = safe_mmap_file(&file)?;
        let fd = safe_borrowed_fd_from_file(&file);
        let pool = shm_proxy.create_pool(fd, i32::try_from(total_size).unwrap_or(i32::MAX), qh, ());

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
            qh,
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
            qh,
            user_data_1,
        );

        let slots = [
            BufferSlot::new(buffer_0, busy_0),
            BufferSlot::new(buffer_1, busy_1),
        ];

        Ok(Self {
            shm: MmappedShm { mmap },
            pool,
            width,
            height,
            slots,
            back_index: 0,
        })
    }

    #[must_use]
    pub fn mmap_mut(&mut self) -> &mut [u8] {
        let frame_size = usize::try_from(self.width.saturating_mul(self.height).saturating_mul(4)).unwrap_or_default();
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
            .mmap_mut()
            .get_mut(offset..end)
            .unwrap_or_default()
    }

    #[must_use]
    pub fn current_buffer(&self) -> &wayland_client::protocol::wl_buffer::WlBuffer {
        self.slots
            .get(self.back_index)
            .map_or_else(|| &self.slots[0].buffer, BufferSlot::buffer)
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
            slot.buffer.destroy();
        }
        self.pool.destroy();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_shm_logic() {
        let tmp = Path::new("/tmp");

        // Test create_shm_file success
        let size = 1024;
        let file = create_shm_file(size, tmp).unwrap();
        let size_u64 = u64::try_from(size).unwrap();
        assert_eq!(file.metadata().unwrap().len(), size_u64);

        // Test mmapped_shm_methods
        let size = 4096;
        let mut shm = MmappedShm::new(size, tmp).unwrap();
        assert_eq!(shm.size(), size);
        assert_eq!(shm.mmap_mut().len(), size);

        // Test mmapped_shm_mut_access
        let mut shm = MmappedShm::new(100, tmp).unwrap();
        let data = shm.mmap_mut();
        data[0] = 42;
        assert_eq!(data[0], 42);

        // Test failure with invalid dir
        let res = create_shm_file(64, Path::new("/non_existent_dir_12345"));
        assert!(res.is_err());
    }

    #[test]
    fn test_create_shm_file_error() {
        let res = create_shm_file(usize::MAX, Path::new("/tmp"));
        assert!(res.is_err());
    }

    #[test]
    fn test_buffer_user_data_and_busy_state() {
        let busy = Arc::new(AtomicBool::new(false));
        let user_data = BufferUserData::new(busy.clone());

        assert!(!user_data.is_busy());
        assert!(!busy.load(Ordering::SeqCst));

        user_data.set_busy(true);
        assert!(user_data.is_busy());
        assert!(busy.load(Ordering::SeqCst));

        let cloned_data = user_data.clone();
        assert!(cloned_data.is_busy());

        user_data.set_busy(false);
        assert!(!user_data.is_busy());
        assert!(!cloned_data.is_busy());
    }

    #[test]
    fn test_double_buffering_memory_slicing_simulation() {
        let tmp = Path::new("/tmp");
        let width = 10u32;
        let height = 10u32;
        let frame_size = usize::try_from(width.saturating_mul(height).saturating_mul(4)).unwrap();
        let total_size = frame_size.saturating_mul(2);

        let mut shm = MmappedShm::new(total_size, tmp).unwrap();
        assert_eq!(shm.size(), total_size);

        // Slot 0 write
        let back_index_0 = 0;
        let offset_0 = back_index_0 * frame_size;
        let slot_0 = &mut shm.mmap_mut()[offset_0..offset_0 + frame_size];
        slot_0.fill(0xAA);

        // Slot 1 write
        let back_index_1 = 1;
        let offset_1 = back_index_1 * frame_size;
        let slot_1 = &mut shm.mmap_mut()[offset_1..offset_1 + frame_size];
        slot_1.fill(0xBB);

        // Verify slot 0 and slot 1 do not overlap or overwrite each other
        let full = shm.mmap_mut();
        assert!(full[..frame_size].iter().all(|&b| b == 0xAA));
        assert!(full[frame_size..total_size].iter().all(|&b| b == 0xBB));
    }
}
