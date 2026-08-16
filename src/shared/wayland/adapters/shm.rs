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

pub struct ShmBuffer {
    shm: MmappedShm,
    pool: WlShmPool,
    width: u32,
    height: u32,
    buffer: wayland_client::protocol::wl_buffer::WlBuffer,
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
    pub fn new<S>(shm_proxy: &WlShm, width: u32, height: u32, qh: &QueueHandle<S>, xdg_runtime_dir: &std::path::Path) -> Result<Self>
    where
        S: wayland_client::Dispatch<wayland_client::protocol::wl_shm_pool::WlShmPool, ()>
            + wayland_client::Dispatch<wayland_client::protocol::wl_buffer::WlBuffer, ()>
            + 'static,
    {
        let frame_size = (width * height * 4) as usize;
        let file = create_shm_file(frame_size, xdg_runtime_dir)?;

        let mmap = safe_mmap_file(&file)?;
        let fd = safe_borrowed_fd_from_file(&file);
        let pool = shm_proxy.create_pool(fd, frame_size as i32, qh, ());

        let buffer = pool.create_buffer(
            0,
            width as i32,
            height as i32,
            (width * 4) as i32,
            wayland_client::protocol::wl_shm::Format::Argb8888,
            qh,
            (),
        );

        Ok(Self {
            shm: MmappedShm { mmap },
            pool,
            width,
            height,
            buffer,
        })
    }

    pub fn mmap_mut(&mut self) -> &mut [u8] {
        let frame_size = (self.width * self.height * 4) as usize;
        &mut self.shm.mmap_mut()[..frame_size]
    }

    pub fn current_buffer(&self) -> &wayland_client::protocol::wl_buffer::WlBuffer {
        &self.buffer
    }

    pub fn swap_buffers(&mut self) {
        // No-op for now, single buffer implementation
    }
}

impl Drop for ShmBuffer {
    fn drop(&mut self) {
        self.buffer.destroy();
        self.pool.destroy();
    }
}

#[cfg(test)]
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
}
