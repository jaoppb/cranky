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

pub struct MmappedShm {
    mmap: MmapMut,
}

impl MmappedShm {
    pub fn new(size: usize) -> Result<Self> {
        let file = create_shm_file(size)?;
        let mmap = safe_mmap_file(&file)?;
        Ok(Self { mmap })
    }

    pub fn mmap_mut(&mut self) -> &mut MmapMut {
        &mut self.mmap
    }

    pub fn size(&self) -> usize {
        self.mmap.len()
    }
}

pub struct ShmBuffer {
    shm: MmappedShm,
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
fn safe_borrowed_fd_from_file(file: &File) -> BorrowedFd<'_> {
    unsafe { BorrowedFd::borrow_raw(file.as_raw_fd()) }
}

impl ShmBuffer {
    pub fn new(
        shm_proxy: &WlShm,
        width: u32,
        height: u32,
        qh: &QueueHandle<CrankyState>,
    ) -> Result<Self> {
        let size = (width * height * 4) as usize;
        let file = create_shm_file(size)?;

        let mmap = safe_mmap_file(&file)?;
        let fd = safe_borrowed_fd_from_file(&file);
        let pool = shm_proxy.create_pool(fd, size as i32, qh, ());

        Ok(Self {
            shm: MmappedShm { mmap },
            pool,
        })
    }

    pub fn mmap_mut(&mut self) -> &mut MmapMut {
        self.shm.mmap_mut()
    }

    pub fn pool(&self) -> &WlShmPool {
        &self.pool
    }

    pub fn size(&self) -> usize {
        self.shm.size()
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
    fn test_shm_env_dependent_logic() {
        // Combined test to avoid race conditions with env var manipulation in parallel tests
        with_env("XDG_RUNTIME_DIR", Some("/tmp"), || {
            // Test create_shm_file success
            let size = 1024;
            let file = create_shm_file(size).unwrap();
            assert_eq!(file.metadata().unwrap().len(), size as u64);

            // Test mmapped_shm_methods
            let size = 4096;
            let mut shm = MmappedShm::new(size).unwrap();
            assert_eq!(shm.size(), size);
            assert_eq!(shm.mmap_mut().len(), size);

            // Test mmapped_shm_mut_access
            let mut shm = MmappedShm::new(100).unwrap();
            let data = shm.mmap_mut();
            data[0] = 42;
            assert_eq!(data[0], 42);
        });

        with_env("XDG_RUNTIME_DIR", None, || {
            // Test create_shm_file failure
            let res = create_shm_file(64);
            assert!(res.is_err());
            assert_eq!(res.unwrap_err().kind(), ErrorKind::NotFound);

            // Test mmapped_shm_new_failure
            let res = MmappedShm::new(1024);
            assert!(res.is_err());
        });
    }

    #[test]
    fn test_create_shm_file_error() {
        let res = create_shm_file(usize::MAX);
        assert!(res.is_err());
    }
}
