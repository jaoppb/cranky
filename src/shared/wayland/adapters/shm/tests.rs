use super::slot::BufferUserData;
use cranky_unsafe::SharedMapping;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

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

    let (mut shm, _file) = SharedMapping::create(total_size, tmp).unwrap();
    assert_eq!(shm.bytes_mut().len(), total_size);

    // Slot 0 write
    let back_index_0 = 0;
    let offset_0 = back_index_0 * frame_size;
    let slot_0 = &mut shm.bytes_mut()[offset_0..offset_0 + frame_size];
    slot_0.fill(0xAA);

    // Slot 1 write
    let back_index_1 = 1;
    let offset_1 = back_index_1 * frame_size;
    let slot_1 = &mut shm.bytes_mut()[offset_1..offset_1 + frame_size];
    slot_1.fill(0xBB);

    // Verify slot 0 and slot 1 do not overlap or overwrite each other
    let full = shm.bytes_mut();
    assert!(full[..frame_size].iter().all(|&byte| byte == 0xAA));
    assert!(
        full[frame_size..total_size]
            .iter()
            .all(|&byte| byte == 0xBB)
    );
}
