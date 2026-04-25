#![allow(unsafe_code)]

use crate::core::CrankyState;
use memmap2::MmapMut;
use std::env;
use std::fs::File;
use std::io::{Error, ErrorKind, Result};
use std::os::unix::io::AsRawFd;
use std::os::unix::io::BorrowedFd;
use std::path::PathBuf;
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::protocol::wl_shm_pool::WlShmPool;

pub struct ShmBuffer {
    mmap: MmapMut,
    pool: WlShmPool,
}

fn create_shm_file(size: usize) -> Result<File> {
    // Use XDG_RUNTIME_DIR for SHM files as per Wayland standards
    let mut path = env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "XDG_RUNTIME_DIR not set"))?;

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
fn safe_borrowed_fd_from_file(file: &File) -> BorrowedFd {
    unsafe { BorrowedFd::borrow_raw(file.as_raw_fd()) }
}

impl ShmBuffer {
    pub fn new(
        shm: &WlShm,
        width: u32,
        height: u32,
        qh: &QueueHandle<CrankyState>,
    ) -> Result<Self> {
        let size = (width * height * 4) as usize;
        let file = create_shm_file(size)?;

        let mmap = safe_mmap_file(&file)?;
        let fd = safe_borrowed_fd_from_file(&file);
        let pool = shm.create_pool(fd, size as i32, qh, ());

        Ok(Self { mmap, pool })
    }

    pub fn mmap_mut(&mut self) -> &mut MmapMut {
        &mut self.mmap
    }

    pub fn pool(&self) -> &WlShmPool {
        &self.pool
    }

    pub fn size(&self) -> usize {
        self.mmap.len()
    }

    #[cfg(test)]
    pub fn test_new(mmap: MmapMut, pool: WlShmPool) -> Self {
        Self { mmap, pool }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_env<F: FnOnce()>(key: &str, value: Option<&str>, f: F) {
        let old_value = env::var_os(key);
        if let Some(val) = value {
            unsafe {
                env::set_var(key, val);
            }
        } else {
            unsafe {
                env::remove_var(key);
            }
        }
        f();
        if let Some(val) = old_value {
            unsafe {
                env::set_var(key, val);
            }
        } else {
            unsafe {
                env::remove_var(key);
            }
        }
    }

    #[test]
    fn test_create_shm_file() {
        with_env("XDG_RUNTIME_DIR", Some("/tmp"), || {
            let size = 1024;
            let file = create_shm_file(size).unwrap();
            assert_eq!(file.metadata().unwrap().len(), size as u64);
        });
    }

    #[test]
    fn test_shm_buffer_methods() {
        with_env("XDG_RUNTIME_DIR", Some("/tmp"), || {
            let size = 4096;
            let file = create_shm_file(size).unwrap();
            let mmap = safe_mmap_file(&file).unwrap();
            let pool = unsafe { std::mem::zeroed::<WlShmPool>() };

            let mut buffer = ShmBuffer { mmap, pool };
            assert_eq!(buffer.size(), size);
            assert_eq!(buffer.mmap_mut().len(), size);

            std::mem::forget(buffer);
        });
    }

    #[test]
    fn test_create_shm_file_error() {
        let res = create_shm_file(usize::MAX);
        assert!(res.is_err());
    }

    #[test]
    fn test_create_shm_file_without_runtime_dir() {
        with_env("XDG_RUNTIME_DIR", None, || {
            let res = create_shm_file(64);
            assert!(res.is_err());
            assert_eq!(res.unwrap_err().kind(), ErrorKind::NotFound);
        });
    }

    #[test]
    fn test_shm_buffer_test_new_constructor() {
        with_env("XDG_RUNTIME_DIR", Some("/tmp"), || {
            let file = create_shm_file(256).unwrap();
            let mmap = safe_mmap_file(&file).unwrap();
            let pool = unsafe { std::mem::zeroed::<WlShmPool>() };
            let buffer = ShmBuffer::test_new(mmap, pool);
            assert_eq!(buffer.size(), 256);
            std::mem::forget(buffer);
        });
    }
}
