use super::mmap::{create_shm_file, MmappedShm};
use super::slot::BufferUserData;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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
