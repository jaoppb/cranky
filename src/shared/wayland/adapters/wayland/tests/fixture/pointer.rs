//! Pointer-routing helpers: a popup with one composited child, a bar module
//! surface, and builders for the expected `(ModuleId, MonitorId, event)`s.

use super::{Fake, popup_kind, solid};
use crate::shared::config::domain::PopupEmbed;
use crate::shared::events::core::{InteractionEvent, PointerButton, PointerEvent, SurfaceKind};
use crate::shared::primitives::{ModuleId, MonitorId, Position, Size};
use crate::shared::wayland::adapters::wayland::state::WaylandState;
use tokio::sync::broadcast::Receiver;
use wayland_client::WEnum;
use wayland_client::protocol::wl_pointer::ButtonState;
use wayland_client::protocol::wl_surface::WlSurface;

pub type Event = (ModuleId, MonitorId, InteractionEvent);

/// `popup_kind()`'s owner: module 1 on DP-1.
pub const OWNER: u32 = 1;
pub const CHILD: u32 = 9;
pub const BAR_MODULE: u32 = 3;
pub const LEFT: u32 = 0x110;
pub const RIGHT: u32 = 0x111;
pub const PRESSED: WEnum<ButtonState> = WEnum::Value(ButtonState::Pressed);
pub const RELEASED: WEnum<ButtonState> = WEnum::Value(ButtonState::Released);

/// A 20×10 popup owned by module 1 on DP-1, whose bar has `scale`, with
/// module 9 composited as a 4×4 logical layer at (10, 2).
pub fn popup(fake: &Fake, scale: u32) -> (WaylandState, WlSurface) {
    let mut state = fake.state(PopupEmbed::Composite);
    state
        .bars
        .push(fake.bar("DP-1", i32::try_from(scale).unwrap_or(1)));
    let mut floating = fake.floating(Size::new(20, 10), solid(20, 10, [0; 4]));
    let side = 4u32.saturating_mul(scale);
    floating.layers.upsert(
        ModuleId::new(CHILD),
        MonitorId::new("DP-1"),
        Position::new(10, 2),
        solid(side, side, [1; 4]),
    );
    let surface = floating.surface.clone();
    state.surface_to_id.insert(
        surface.clone(),
        (
            ModuleId::new(OWNER),
            MonitorId::new("DP-1"),
            SurfaceKind::Popup,
        ),
    );
    state.floating_surfaces.insert(popup_kind(), floating);
    (state, surface)
}

/// Registers a bar module subsurface for module 3 on HDMI-A-1.
pub fn bar_module(fake: &Fake, state: &mut WaylandState) -> WlSurface {
    let surface = fake.surface();
    state.surface_to_id.insert(
        surface.clone(),
        (
            ModuleId::new(BAR_MODULE),
            MonitorId::new("HDMI-A-1"),
            SurfaceKind::Bar,
        ),
    );
    surface
}

pub fn drain(rx: &mut Receiver<Event>) -> Vec<Event> {
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

pub fn ev(id: u32, monitor: &str, event: PointerEvent) -> Event {
    (
        ModuleId::new(id),
        MonitorId::new(monitor),
        InteractionEvent::Pointer(event),
    )
}

pub fn owner(event: PointerEvent) -> Event {
    ev(OWNER, "DP-1", event)
}

pub fn child(event: PointerEvent) -> Event {
    ev(CHILD, "DP-1", event)
}

pub fn bar(event: PointerEvent) -> Event {
    ev(BAR_MODULE, "HDMI-A-1", event)
}

pub const fn enter(surface: SurfaceKind) -> PointerEvent {
    PointerEvent::PointerEnter { surface }
}

pub const fn leave(surface: SurfaceKind) -> PointerEvent {
    PointerEvent::PointerLeave { surface }
}

pub const fn motion(surface: SurfaceKind, x: i32, y: i32) -> PointerEvent {
    PointerEvent::PointerMotion {
        surface,
        pos: Position::new(x, y),
    }
}

pub const fn press(surface: SurfaceKind, x: i32, y: i32) -> PointerEvent {
    PointerEvent::ButtonPress {
        surface,
        button: PointerButton::Left,
        pos: Position::new(x, y),
    }
}

/// The `ButtonRelease` + `Click` pair one left-button release produces.
pub fn release(surface: SurfaceKind, x: i32, y: i32) -> [PointerEvent; 2] {
    let (button, pos) = (PointerButton::Left, Position::new(x, y));
    [
        PointerEvent::ButtonRelease {
            surface,
            button,
            pos,
        },
        PointerEvent::Click {
            surface,
            button,
            pos,
        },
    ]
}

/// Press, release and click of the left button at one position.
pub fn clicks(surface: SurfaceKind, x: i32, y: i32) -> [PointerEvent; 3] {
    let [up, click] = release(surface, x, y);
    [press(surface, x, y), up, click]
}
