use cranky_unsafe::SharedMapping;
use std::fs::File;
use std::io::Result;

pub struct MmappedShm {
    mmap: SharedMapping,
}

impl MmappedShm {
    /// # Errors
    ///
    /// Returns an I/O error if memory-mapping the file fails.
    pub(crate) fn from_file(file: &File) -> Result<Self> {
        Ok(Self {
            mmap: SharedMapping::map(file)?,
        })
    }

    #[cfg(test)]
    /// # Errors
    ///
    /// Returns an I/O error if creating or memory-mapping the SHM file fails.
    pub fn new(size: usize, xdg_runtime_dir: &std::path::Path) -> Result<Self> {
        Self::from_file(&create_shm_file(size, xdg_runtime_dir)?)
    }

    #[must_use]
    pub fn mmap_mut(&mut self) -> &mut [u8] {
        self.mmap.bytes_mut()
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
