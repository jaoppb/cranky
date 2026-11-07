//! Keyboard input state: the `wl_keyboard`, the surface holding keyboard
//! focus, and the key codes the current keymap maps to Escape.
//!
//! Raw key codes are not enough to recognise Esc: compositors apply layout
//! options (`caps:escape`, …) by shipping a remapped keymap, so the physical
//! key and the keysym differ. The keymap arrives as a file descriptor, which
//! is read as text (`new_from_string`) rather than mmapped (which would need
//! `unsafe`). It is then reduced to the set of Escape key codes up front,
//! because xkb handles are not `Send` and the state must live in the adapter.

use std::io::Read;
use std::os::fd::OwnedFd;
use wayland_client::protocol::{wl_keyboard::WlKeyboard, wl_seat::WlSeat, wl_surface::WlSurface};
use wayland_client::{Dispatch, Proxy, QueueHandle};
use xkbcommon::xkb;

/// evdev key codes are offset by 8 in xkb.
const EVDEV_OFFSET: u32 = 8;

#[derive(Default)]
pub struct KeyboardInput {
    device: Option<WlKeyboard>,
    focus: Option<WlSurface>,
    /// evdev codes of every key that produces Escape in some layout or level.
    escape_codes: Vec<u32>,
}

impl KeyboardInput {
    /// Follows the seat's keyboard capability: binds a `wl_keyboard` when it
    /// appears, releases it and forgets everything about it when it goes.
    pub(crate) fn on_capability<D>(&mut self, seat: &WlSeat, present: bool, qh: &QueueHandle<D>)
    where
        D: Dispatch<WlKeyboard, ()> + 'static,
    {
        if present && self.device.is_none() {
            self.device = Some(seat.get_keyboard(qh, ()));
        } else if !present && let Some(device) = self.device.take() {
            if device.version() >= 3 {
                device.release();
            }
            *self = Self::default();
        }
    }

    pub(crate) const fn focus(&self) -> Option<&WlSurface> {
        self.focus.as_ref()
    }

    pub(crate) fn set_focus(&mut self, surface: WlSurface) {
        self.focus = Some(surface);
    }

    /// Drops the focus if it is `surface`: it is being destroyed, or the
    /// compositor said the keyboard left it. A `Leave` for a surface that no
    /// longer holds focus must not clear a newer `Enter`.
    pub(crate) fn forget_focus(&mut self, surface: &WlSurface) {
        if self.focus.as_ref() == Some(surface) {
            self.focus = None;
        }
    }

    /// Replaces the keymap with the text read from `fd`; on any failure no
    /// key is recognised until the next keymap.
    pub(crate) fn load_keymap(&mut self, fd: OwnedFd, size: u32) {
        self.escape_codes = read_keymap(fd, size)
            .map(|keymap| escape_codes(&keymap))
            .unwrap_or_default();
    }

    #[cfg(test)]
    pub(crate) fn with_escape_code(code: u32) -> Self {
        Self {
            escape_codes: vec![code],
            ..Self::default()
        }
    }

    /// Whether the evdev key `code` produces the Escape keysym.
    pub(crate) fn is_escape(&self, code: u32) -> bool {
        self.escape_codes.contains(&code)
    }
}

fn read_keymap(fd: OwnedFd, size: u32) -> Option<xkb::Keymap> {
    let mut bytes = Vec::new();
    std::fs::File::from(fd)
        .take(u64::from(size))
        .read_to_end(&mut bytes)
        .ok()?;
    // The protocol terminates the text with a NUL.
    while bytes.last() == Some(&0) {
        bytes.pop();
    }
    let text = String::from_utf8(bytes).ok()?;
    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    xkb::Keymap::new_from_string(
        &context,
        text,
        xkb::KEYMAP_FORMAT_TEXT_V1,
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    )
}

fn escape_codes(keymap: &xkb::Keymap) -> Vec<u32> {
    let (min, max) = (keymap.min_keycode().raw(), keymap.max_keycode().raw());
    (min..=max)
        .filter(|&raw| {
            let key = xkb::Keycode::new(raw);
            (0..keymap.num_layouts_for_key(key)).any(|layout| {
                (0..keymap.num_levels_for_key(key, layout)).any(|level| {
                    keymap
                        .key_get_syms_by_level(key, layout, level)
                        .contains(&xkb::Keysym::Escape)
                })
            })
        })
        .filter_map(|raw| raw.checked_sub(EVDEV_OFFSET))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    const ESC_EVDEV: u32 = 1;
    const CAPS_EVDEV: u32 = 58;

    /// A read-only fd over `bytes`, backed by an already-unlinked temp file.
    fn fd_with(bytes: &[u8]) -> OwnedFd {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("cranky-keymap-{}-{n}", std::process::id()));
        std::fs::write(&path, bytes).unwrap();
        let file = std::fs::File::open(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        OwnedFd::from(file)
    }

    /// A keymap file as a compositor would send it, built from layout
    /// `options` (e.g. `caps:escape`).
    fn keymap_fd(options: &str) -> (OwnedFd, u32) {
        let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let keymap = xkb::Keymap::new_from_names(
            &context,
            "",
            "",
            "us",
            "",
            Some(options.to_string()),
            xkb::KEYMAP_COMPILE_NO_FLAGS,
        )
        .expect("us layout is available");
        let mut text = keymap
            .get_as_string(xkb::KEYMAP_FORMAT_TEXT_V1)
            .into_bytes();
        text.push(0);
        let size = u32::try_from(text.len()).unwrap();
        (fd_with(&text), size)
    }

    fn input_with(options: &str) -> KeyboardInput {
        let (fd, size) = keymap_fd(options);
        let mut input = KeyboardInput::default();
        input.load_keymap(fd, size);
        input
    }

    #[test]
    fn test_escape_key_is_recognised() {
        let input = input_with("");
        assert!(input.is_escape(ESC_EVDEV));
        assert!(!input.is_escape(CAPS_EVDEV));
    }

    #[test]
    fn test_remapped_escape_follows_the_keymap_not_the_physical_key() {
        let input = input_with("caps:escape");
        assert!(input.is_escape(CAPS_EVDEV));
    }

    #[test]
    fn test_no_keymap_recognises_nothing() {
        assert!(!KeyboardInput::default().is_escape(ESC_EVDEV));
    }

    #[test]
    fn test_garbage_keymap_leaves_no_keymap() {
        let mut input = input_with("");
        input.load_keymap(fd_with(b"not a keymap\0"), 13);
        assert!(!input.is_escape(ESC_EVDEV));
    }
}
