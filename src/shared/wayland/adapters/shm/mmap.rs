#![allow(unsafe_code)]

use memmap2::MmapMut;
use std::fs::File;
use std::io::Result;
use std::os::unix::io::{AsRawFd, BorrowedFd};

pub struct MmappedShm {
    mmap: MmapMut,
}

impl MmappedShm {
    pub(crate) const fn from_mmap(mmap: MmapMut) -> Self {
        Self { mmap }
    }

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

pub(crate) fn create_shm_file(size: usize, xdg_runtime_dir: &std::path::Path) -> Result<File> {
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
    let len = u64::try_from(size)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    file.set_len(len)?;
    Ok(file)
}

// Safe wrapper around unsafe mmap creation
pub(crate) fn safe_mmap_file(file: &File) -> Result<MmapMut> {
    unsafe { MmapMut::map_mut(file) }
}

// Safe wrapper around unsafe BorrowedFd creation for file descriptors
pub(crate) fn safe_borrowed_fd_from_file(file: &File) -> BorrowedFd<'_> {
    unsafe { BorrowedFd::borrow_raw(file.as_raw_fd()) }
}
