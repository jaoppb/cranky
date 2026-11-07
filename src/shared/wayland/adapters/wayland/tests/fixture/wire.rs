//! Decodes the requests the client wrote to the fixture's socket, so a test
//! can assert what would have reached a compositor.

use super::Fake;
use std::io::Read;
use wayland_client::Proxy;

/// `wl_compositor.create_region` / `wl_surface.set_input_region` /
/// `wl_region.{destroy,add}`.
pub const CREATE_REGION: u16 = 1;
pub const SET_INPUT_REGION: u16 = 5;
pub const REGION_DESTROY: u16 = 0;
pub const REGION_ADD: u16 = 1;

/// One decoded request: the object it was sent to, its opcode and the
/// first argument (the new id for a constructor, the region id for
/// `set_input_region`; `0` when there are no arguments).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request {
    pub object: u32,
    pub opcode: u16,
    pub arg: u32,
}

impl Fake {
    /// Every request flushed since the last call, in send order.
    pub fn requests(&self) -> Vec<Request> {
        self.connection.flush().expect("flush");
        self.server.set_nonblocking(true).expect("nonblocking");
        let mut bytes = Vec::new();
        let mut chunk = [0u8; 4096];
        while let Ok(n) = (&self.server).read(&mut chunk) {
            if n == 0 {
                break;
            }
            bytes.extend_from_slice(&chunk[..n]);
        }
        let word = |at: usize| {
            bytes
                .get(at..at.checked_add(4)?)
                .and_then(|w| <[u8; 4]>::try_from(w).ok())
                .map(u32::from_ne_bytes)
        };
        let mut out = Vec::new();
        let mut at = 0usize;
        while let (Some(object), Some(head)) = (word(at), at.checked_add(4).and_then(word)) {
            let size = usize::try_from(head >> 16).expect("size");
            out.push(Request {
                object,
                opcode: u16::try_from(head & 0xffff).expect("opcode"),
                arg: at.checked_add(8).and_then(word).unwrap_or(0),
            });
            at = at.saturating_add(size.max(8));
        }
        out
    }
}

/// The wire id of a protocol object.
pub fn id(proxy: &impl Proxy) -> u32 {
    proxy.id().protocol_id()
}
