use memmap2::MmapMut;
use std::fs::File;
use std::io::Result;
use std::path::Path;

/// A writable, shared memory mapping of an anonymous file.
#[derive(Debug)]
pub struct SharedMapping {
    mmap: MmapMut,
}

impl SharedMapping {
    /// Creates a file of `len` bytes in `dir`, unlinks it, and maps it.
    ///
    /// Returns the mapping together with the open file, whose descriptor is
    /// what a Wayland buffer pool is created from.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the file cannot be created, sized or mapped.
    pub fn create(len: usize, dir: &Path) -> Result<(Self, File)> {
        let file = create_unlinked_file(len, dir)?;
        // SAFETY: a mapping is undefined behaviour if the file is changed
        // through another mapping or truncated while it is mapped. The file was
        // created and unlinked just above, so no other process can open it by
        // path. Its descriptor leaves this function only through the returned
        // `File`, which Cranky hands to the compositor as a `wl_shm` pool; the
        // compositor reads from the pool and never resizes it.
        let mmap = unsafe { MmapMut::map_mut(&file) }?;
        Ok((Self { mmap }, file))
    }

    /// The mapped bytes.
    #[must_use]
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        &mut self.mmap
    }
}

fn create_unlinked_file(len: usize, dir: &Path) -> Result<File> {
    let path = dir.join(format!("cranky-shm-{}", uuid::Uuid::new_v4()));

    let file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)?;

    // Immediately unlink the file so it's only accessible via the FD
    let _ = std::fs::remove_file(&path);
    let len =
        u64::try_from(len).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    file.set_len(len)?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::SharedMapping;
    use std::io::{Read, Seek, SeekFrom};
    use std::path::Path;

    #[test]
    fn the_mapping_is_as_long_as_requested_and_backed_by_the_returned_file() {
        let (mut mapping, mut file) = SharedMapping::create(8, &std::env::temp_dir()).unwrap();
        assert_eq!(file.metadata().unwrap().len(), 8);
        assert_eq!(mapping.bytes_mut().len(), 8);

        mapping
            .bytes_mut()
            .copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);

        let mut read_back = Vec::new();
        file.seek(SeekFrom::Start(0)).unwrap();
        file.read_to_end(&mut read_back).unwrap();
        assert_eq!(read_back, [1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn a_missing_directory_is_an_error() {
        assert!(SharedMapping::create(64, Path::new("/non_existent_dir_12345")).is_err());
    }

    #[test]
    fn a_length_the_filesystem_cannot_hold_is_an_error() {
        assert!(SharedMapping::create(usize::MAX, &std::env::temp_dir()).is_err());
    }
}
