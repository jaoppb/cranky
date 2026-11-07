//! `wl_keyboard` event handling. The only key acted on is Esc, which closes
//! the popup holding keyboard focus — the same teardown a compositor
//! `PopupDone` triggers, so its nested popups go with it.
//!
//! The compositor only focuses a popup that took an `xdg_popup` grab, so Esc
//! works on exactly those; any other focus (the bar, a panel) is ignored.

use super::floating_teardown::teardown_floating;
use super::state::WaylandState;
use crate::features::layout_engine::domain::FloatingKind;
use wayland_client::protocol::wl_keyboard::{self, KeyState, WlKeyboard};
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};

impl Dispatch<WlKeyboard, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _proxy: &WlKeyboard,
        event: wl_keyboard::Event,
        (): &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Keymap { fd, size, .. } => state.keyboard.load_keymap(fd, size),
            wl_keyboard::Event::Enter { surface, .. } => state.keyboard.set_focus(surface),
            wl_keyboard::Event::Leave { .. } => state.keyboard.clear_focus(),
            wl_keyboard::Event::Key {
                key,
                state: WEnum::Value(KeyState::Pressed),
                ..
            } => {
                handle_key(state, key);
            }
            _ => {}
        }
    }
}

/// Closes the focused popup if `code` is Esc. Returns whether it did.
pub(super) fn handle_key(state: &mut WaylandState, code: u32) -> bool {
    if !state.keyboard.is_escape(code) {
        return false;
    }
    let Some(focus) = state.keyboard.focus() else {
        return false;
    };
    let popup = state
        .floating_surfaces
        .iter()
        .find(|(kind, floating)| {
            matches!(kind, FloatingKind::Popup(_)) && &floating.surface == focus
        })
        .map(|(kind, _)| kind.clone());
    let Some(kind) = popup else {
        return false;
    };
    teardown_floating(state, &kind, true);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::layout_engine::domain::PopupTarget;
    use crate::shared::events::core::{InteractionEvent, SurfaceLifecycleEvent};
    use crate::shared::primitives::{ModuleId, MonitorId, Size};
    use crate::shared::wayland::adapters::wayland::keyboard::KeyboardInput;
    use crate::shared::wayland::adapters::wayland::tests::fixture::pointer::{
        CHILD, Event, OWNER, bar_module, drain, popup,
    };
    use crate::shared::wayland::adapters::wayland::tests::fixture::{Fake, popup_kind, solid};

    const ESC: u32 = 1;
    const OTHER: u32 = 30;

    fn dismissed(module: u32) -> Event {
        let event = InteractionEvent::Lifecycle(SurfaceLifecycleEvent::PopupDismissed);
        (ModuleId::new(module), MonitorId::new("DP-1"), event)
    }

    fn nested_kind() -> FloatingKind {
        FloatingKind::Popup(PopupTarget::new(
            ModuleId::new(CHILD),
            MonitorId::new("DP-1"),
        ))
    }

    /// `popup_kind()` open with a popup nested inside it; keyboard focus on
    /// the outer one.
    fn nested(fake: &Fake) -> WaylandState {
        let (mut state, outer) = popup(fake, 1);
        let mut inner = fake.floating(Size::new(1, 1), solid(1, 1, [0; 4]));
        inner.parent = Some(popup_kind());
        state.floating_surfaces.insert(nested_kind(), inner);
        state.keyboard = KeyboardInput::with_escape_code(ESC);
        state.keyboard.set_focus(outer);
        state
    }

    fn open(state: &WaylandState) -> Vec<FloatingKind> {
        let mut kinds: Vec<_> = state.floating_surfaces.keys().cloned().collect();
        kinds.sort_by_key(|k| format!("{k:?}"));
        kinds
    }

    #[test]
    fn esc_closes_the_focused_popup_and_tells_its_owner() {
        let fake = Fake::new();
        let (mut state, surface) = popup(&fake, 1);
        state.keyboard = KeyboardInput::with_escape_code(ESC);
        state.keyboard.set_focus(surface);
        let mut rx = state.hub.pointer_rx();
        assert!(handle_key(&mut state, ESC));
        assert!(state.floating_surfaces.is_empty());
        assert_eq!(drain(&mut rx), vec![dismissed(OWNER)]);
    }

    #[test]
    fn esc_closes_nested_popups_with_their_parent_and_notifies_both() {
        let fake = Fake::new();
        let mut state = nested(&fake);
        let mut rx = state.hub.pointer_rx();
        assert!(handle_key(&mut state, ESC));
        assert!(state.floating_surfaces.is_empty());
        assert_eq!(drain(&mut rx), vec![dismissed(CHILD), dismissed(OWNER)]);
    }

    #[test]
    fn esc_on_a_focused_nested_popup_leaves_its_parent_open() {
        let fake = Fake::new();
        let mut state = nested(&fake);
        let inner = state.floating_surfaces[&nested_kind()].surface.clone();
        state.keyboard.set_focus(inner);
        assert!(handle_key(&mut state, ESC));
        assert_eq!(open(&state), vec![popup_kind()]);
    }

    #[test]
    fn other_keys_close_nothing() {
        let fake = Fake::new();
        let mut state = nested(&fake);
        assert!(!handle_key(&mut state, OTHER));
        assert_eq!(state.floating_surfaces.len(), 2);
    }

    #[test]
    fn esc_without_keyboard_focus_closes_nothing() {
        let fake = Fake::new();
        let mut state = nested(&fake);
        state.keyboard.clear_focus();
        assert!(!handle_key(&mut state, ESC));
        assert_eq!(state.floating_surfaces.len(), 2);
    }

    #[test]
    fn esc_with_focus_on_the_bar_closes_nothing() {
        let fake = Fake::new();
        let mut state = nested(&fake);
        let bar = bar_module(&fake, &mut state);
        state.keyboard.set_focus(bar);
        assert!(!handle_key(&mut state, ESC));
        assert_eq!(state.floating_surfaces.len(), 2);
    }

    #[test]
    fn esc_with_focus_on_a_panel_closes_nothing() {
        let fake = Fake::new();
        let mut state = nested(&fake);
        let panel =
            FloatingKind::Panel(PopupTarget::new(ModuleId::new(77), MonitorId::new("DP-1")));
        let floating = fake.floating(Size::new(1, 1), solid(1, 1, [0; 4]));
        let surface = floating.surface.clone();
        state.floating_surfaces.insert(panel.clone(), floating);
        state.keyboard.set_focus(surface);
        assert!(!handle_key(&mut state, ESC));
        assert!(state.floating_surfaces.contains_key(&panel));
    }

    #[test]
    fn tearing_down_the_focused_popup_drops_the_focus() {
        let fake = Fake::new();
        let mut state = nested(&fake);
        teardown_floating(&mut state, &popup_kind(), true);
        assert!(state.keyboard.focus().is_none());
    }
}
