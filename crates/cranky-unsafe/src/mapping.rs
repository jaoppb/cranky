use memmap2::MmapMut;
use std::fs::File;
use std::io::Result;

/// A writable, shared memory mapping of a whole file.
#[derive(Debug)]
pub struct SharedMapping {
    mmap: MmapMut,
}

impl SharedMapping {
    /// Maps `file` into memory for reading and writing.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the kernel refuses the mapping.
    pub fn map(file: &File) -> Result<Self> {
        // SAFETY: the mapping is only undefined behaviour if the file is
        // truncated while mapped. Cranky maps a file it has just created and
        // unlinked, so no other process can open it by path, and the compositor
        // that receives its descriptor only reads from it.
        let mmap = unsafe { MmapMut::map_mut(file) }?;
        Ok(Self { mmap })
    }

    /// The mapped bytes.
    #[must_use]
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        &mut self.mmap
    }

    /// The length of the mapping in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.mmap.len()
    }

    /// Whether the mapping is zero bytes long.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.mmap.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::SharedMapping;
    use std::io::{Read, Seek, SeekFrom, Write};

    fn file_of(len: u64) -> std::fs::File {
        let path = std::env::temp_dir().join(format!("cranky-unsafe-{}-{len}", std::process::id()));
        let mut file = std::fs::File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .unwrap();
        let _ = std::fs::remove_file(&path);
        file.set_len(len).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn writes_through_the_mapping_reach_the_file() {
        let mut file = file_of(8);
        let mut mapping = SharedMapping::map(&file).unwrap();
        assert_eq!(mapping.len(), 8);
        assert!(!mapping.is_empty());

        mapping
            .bytes_mut()
            .copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);

        let mut read_back = Vec::new();
        file.seek(SeekFrom::Start(0)).unwrap();
        file.read_to_end(&mut read_back).unwrap();
        assert_eq!(read_back, [1, 2, 3, 4, 5, 6, 7, 8]);
    }
}
