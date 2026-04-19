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

impl ShmBuffer {
    pub fn new(
        shm: &WlShm,
        width: u32,
        height: u32,
        qh: &QueueHandle<CrankyState>,
    ) -> Result<Self> {
        let size = (width * height * 4) as usize;
        let file = create_shm_file(size)?;

        let mmap = unsafe { MmapMut::map_mut(&file)? };

        let fd = unsafe { BorrowedFd::borrow_raw(file.as_raw_fd()) };
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

    #[test]
    fn test_create_shm_file() {
        // Ensure XDG_RUNTIME_DIR is set for the test
        if env::var_os("XDG_RUNTIME_DIR").is_none() {
            unsafe {
                env::set_var("XDG_RUNTIME_DIR", "/tmp");
            }
        }

        let size = 1024;
        let file = create_shm_file(size).unwrap();
        assert_eq!(file.metadata().unwrap().len(), size as u64);
    }

    #[test]
    fn test_shm_buffer_methods() {
        if env::var_os("XDG_RUNTIME_DIR").is_none() {
            unsafe {
                env::set_var("XDG_RUNTIME_DIR", "/tmp");
            }
        }
        let size = 4096;
        let file = create_shm_file(size).unwrap();
        let mmap = unsafe { MmapMut::map_mut(&file).unwrap() };
        let pool = unsafe { std::mem::MaybeUninit::<WlShmPool>::uninit().assume_init() };

        let mut buffer = ShmBuffer { mmap, pool };
        assert_eq!(buffer.size(), size);
        assert_eq!(buffer.mmap_mut().len(), size);

        std::mem::forget(buffer); // Avoid dropping uninitialized WlShmPool
    }

    #[test]
    fn test_create_shm_file_error() {
        let res = create_shm_file(usize::MAX); // Should fail to allocate or similar
        assert!(res.is_err());
    }

    #[test]
    fn test_create_shm_file_without_runtime_dir() {
        let old = env::var_os("XDG_RUNTIME_DIR");
        unsafe {
            env::remove_var("XDG_RUNTIME_DIR");
        }
        let res = create_shm_file(64);
        if let Some(val) = old {
            unsafe {
                env::set_var("XDG_RUNTIME_DIR", val);
            }
        }
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().kind(), ErrorKind::NotFound);
    }

    #[test]
    fn test_shm_buffer_new_propagates_create_file_error() {
        let old = env::var_os("XDG_RUNTIME_DIR");
        unsafe {
            env::remove_var("XDG_RUNTIME_DIR");
        }

        let shm = unsafe { std::mem::MaybeUninit::<WlShm>::uninit().assume_init() };
        let qh =
            unsafe { std::mem::MaybeUninit::<QueueHandle<CrankyState>>::uninit().assume_init() };
        let res = ShmBuffer::new(&shm, 4, 4, &qh);

        if let Some(val) = old {
            unsafe {
                env::set_var("XDG_RUNTIME_DIR", val);
            }
        }
        assert!(res.is_err());
        std::mem::forget(shm);
        std::mem::forget(qh);
    }

    #[test]
    fn test_shm_buffer_test_new_constructor() {
        if env::var_os("XDG_RUNTIME_DIR").is_none() {
            unsafe {
                env::set_var("XDG_RUNTIME_DIR", "/tmp");
            }
        }
        let file = create_shm_file(256).unwrap();
        let mmap = unsafe { MmapMut::map_mut(&file).unwrap() };
        let pool = unsafe { std::mem::MaybeUninit::<WlShmPool>::uninit().assume_init() };
        let buffer = ShmBuffer::test_new(mmap, pool);
        assert_eq!(buffer.size(), 256);
        std::mem::forget(buffer);
    }
}
