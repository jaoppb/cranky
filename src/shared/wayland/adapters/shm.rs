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
    pub fn new(size: usize, xdg_runtime_dir: &std::path::Path) -> Result<Self> {
        let file = create_shm_file(size, xdg_runtime_dir)?;
        let mmap = safe_mmap_file(&file)?;
        Ok(Self { mmap })
    }

    pub fn mmap_mut(&mut self) -> &mut [u8] {
        &mut self.mmap
    }

    #[cfg(test)]
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
    pub fn new(busy: Arc<AtomicBool>) -> Self {
        Self { busy }
    }

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
    pub fn new(buffer: wayland_client::protocol::wl_buffer::WlBuffer, busy: Arc<AtomicBool>) -> Self {
        Self { buffer, busy }
    }

    pub fn buffer(&self) -> &wayland_client::protocol::wl_buffer::WlBuffer {
        &self.buffer
    }

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

    path.push(format!("cranky-shm-{}", uuid::Uuid::new_v4()));

    let file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)?;

    // Immediately unlink the file so it's only accessible via the FD
    let _ = std::fs::remove_file(&path);
    file.set_len(size as u64)?;
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
        let frame_size = (width * height * 4) as usize;
        let total_size = frame_size * 2;
        let file = create_shm_file(total_size, xdg_runtime_dir)?;

        let mmap = safe_mmap_file(&file)?;
        let fd = safe_borrowed_fd_from_file(&file);
        let pool = shm_proxy.create_pool(fd, total_size as i32, qh, ());

        let busy_0 = Arc::new(AtomicBool::new(false));
        let user_data_0 = BufferUserData::new(busy_0.clone());
        let buffer_0 = pool.create_buffer(
            0,
            width as i32,
            height as i32,
            (width * 4) as i32,
            wayland_client::protocol::wl_shm::Format::Argb8888,
            qh,
            user_data_0,
        );

        let busy_1 = Arc::new(AtomicBool::new(false));
        let user_data_1 = BufferUserData::new(busy_1.clone());
        let buffer_1 = pool.create_buffer(
            frame_size as i32,
            width as i32,
            height as i32,
            (width * 4) as i32,
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

    pub fn mmap_mut(&mut self) -> &mut [u8] {
        let frame_size = (self.width * self.height * 4) as usize;
        let offset = self.back_index * frame_size;
        if self.slots[self.back_index].is_busy() {
            tracing::debug!(
                slot = self.back_index,
                "Target back buffer is still marked busy by compositor; writing anyway"
            );
        }
        &mut self.shm.mmap_mut()[offset..offset + frame_size]
    }

    pub fn current_buffer(&self) -> &wayland_client::protocol::wl_buffer::WlBuffer {
        self.slots[self.back_index].buffer()
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn swap_buffers(&mut self) {
        self.slots[self.back_index].set_busy(true);
        self.back_index = 1 - self.back_index;
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
        assert_eq!(file.metadata().unwrap().len(), size as u64);

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
        let frame_size = (width * height * 4) as usize;
        let total_size = frame_size * 2;

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
