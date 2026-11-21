//! Double-buffered `wl_shm` pixel buffers that bar, module and popup surfaces attach.
pub mod buffer;
pub mod slot;
#[cfg(test)]
mod tests;

pub(super) use buffer::ShmBuffer;
pub(super) use slot::BufferUserData;
