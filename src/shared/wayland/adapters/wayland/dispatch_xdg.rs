use super::state::WaylandState;
use super::types::FloatingSurface;
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::xdg::shell::client::{
    xdg_popup::{self, XdgPopup},
    xdg_positioner::{self, XdgPositioner},
    xdg_surface::{self, XdgSurface},
    xdg_wm_base::{self, XdgWmBase},
};

impl Dispatch<XdgWmBase, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        proxy: &XdgWmBase,
        event: xdg_wm_base::Event,
        (): &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            proxy.pong(serial);
        }
    }
}

impl Dispatch<XdgSurface, ()> for WaylandState {
    fn event(
        state: &mut Self,
        proxy: &XdgSurface,
        event: xdg_surface::Event,
        (): &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            on_xdg_surface_configure(state, proxy, serial);
        }
    }
}

/// Acknowledges configure `serial` of `xdg_surface`, then repaints the
/// floating surface that owns it.
fn on_xdg_surface_configure(state: &mut WaylandState, xdg_surface: &XdgSurface, serial: u32) {
    tracing::debug!("xdg_surface Configure serial={serial}");
    xdg_surface.ack_configure(serial);
    repaint_owner_of(state, xdg_surface);
}

/// Finds the floating surface that owns `xdg_surface` and marks it
/// configured and repaints it, or logs that none does.
fn repaint_owner_of(state: &mut WaylandState, xdg_surface: &XdgSurface) {
    let Some(floating) = state
        .floating_surfaces
        .values_mut()
        .find(|floating| &floating.xdg_surface == xdg_surface)
    else {
        tracing::debug!("Configure event for unknown xdg_surface");
        return;
    };
    mark_configured_and_repaint(floating);
}

/// Opens `floating`'s configure gate and repaints it at its own scale.
fn mark_configured_and_repaint(floating: &mut FloatingSurface) {
    // Re-blend rather than attach the back slot as-is: after any
    // commit it holds the previous frame, after a resize zeros.
    tracing::debug!("Repainting floating surface on configure");
    floating.configure.mark_configured();
    let scale = floating.scale;
    super::floating_composite::repaint(floating, scale);
}

impl Dispatch<XdgPopup, ()> for WaylandState {
    fn event(
        state: &mut Self,
        proxy: &XdgPopup,
        event: xdg_popup::Event,
        (): &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            xdg_popup::Event::Repositioned { token } => log_popup_repositioned(token),
            xdg_popup::Event::PopupDone => on_popup_done(state, proxy),
            other => log_unhandled_popup_event(&other),
        }
    }
}

/// Logs the compositor's answer to a reposition under `token`; nothing
/// else follows from it.
fn log_popup_repositioned(token: u32) {
    tracing::debug!("Floating surface popup repositioned (token={token})");
}

/// The compositor dismissed `xdg_popup`: tears its floating surface down.
fn on_popup_done(state: &mut WaylandState, xdg_popup: &XdgPopup) {
    tracing::debug!("xdg_popup Event: PopupDone");
    let Some(kind) = state
        .floating_surfaces
        .iter()
        .find(|(_, floating)| &floating.xdg_popup == xdg_popup)
        .map(|(kind, _)| kind.clone())
    else {
        return;
    };
    super::floating_teardown::teardown_floating(
        state,
        &kind,
        super::floating_teardown::TeardownCause::External,
    );
}

/// Logs a popup event the adapter takes no action on.
fn log_unhandled_popup_event(event: &xdg_popup::Event) {
    tracing::debug!("xdg_popup Event: {event:?}");
}

impl Dispatch<XdgPositioner, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &XdgPositioner,
        _event: xdg_positioner::Event,
        (): &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

#[cfg(test)]
mod tests {
    use crate::features::layout_engine::domain::FloatingKind;
    use crate::shared::config::domain::PopupEmbed;
    use crate::shared::primitives::render::{ModuleFrame, PlacedBuffer, RenderBuffer};
    use crate::shared::primitives::{ModuleId, MonitorId, PopupOffset, Position, Size};
    use crate::shared::wayland::adapters::wayland::floating_anchor::{
        AnchorInfo, FloatingParentRole,
    };
    use crate::shared::wayland::adapters::wayland::floating_composite::repaint;
    use crate::shared::wayland::adapters::wayland::floating_setup::{
        FloatingFrame, FloatingPlacement, update_existing_floating,
    };
    use crate::shared::wayland::adapters::wayland::state::WaylandState;
    use crate::shared::wayland::adapters::wayland::tests::fixture::wire::id;
    use crate::shared::wayland::adapters::wayland::tests::fixture::{
        Fake, committed_bytes, popup_kind, solid,
    };
    use crate::shared::wayland::adapters::wayland::types::FloatingSurface;
    use wayland_protocols::xdg::shell::client::xdg_popup;
    use wayland_protocols::xdg::shell::client::xdg_surface;

    /// `xdg_surface.ack_configure`.
    const ACK_CONFIGURE: u16 = 4;

    const BASE: [u8; 4] = [5, 5, 5, 255];
    /// A 2×1 `BASE` with the 1×1 child at logical (1, 0).
    const BLENDED: [u8; 8] = [5, 5, 5, 255, 1, 2, 3, 255];

    fn with_child(mut floating: FloatingSurface) -> FloatingSurface {
        floating.layers.upsert(ModuleFrame::new(
            ModuleId::new(9),
            MonitorId::new("DP-1"),
            PlacedBuffer::new(Position::new(1, 0), solid(1, 1, [1, 2, 3, 255])),
        ));
        floating
    }

    /// An owner frame `base` for `configured`'s popup, placed at `base`'s
    /// own size.
    fn owner_frame(fake: &Fake, state: &mut WaylandState, base: RenderBuffer) {
        let anchor = AnchorInfo {
            bar_scale: 1,
            parent_role: FloatingParentRole::Popup(fake.xdg_surface()),
            anchor_x: 0,
            anchor_y: 0,
            anchor_w: 1,
            anchor_h: 1,
            target_monitor_id: MonitorId::new("DP-1"),
        };
        let placement = FloatingPlacement {
            anchor,
            text_w: 2,
            text_h: 1,
            offset: PopupOffset::default(),
        };
        let frame = FloatingFrame {
            kind: popup_kind(),
            parent: None,
            buffer: base,
            placement,
        };
        update_existing_floating(state, &fake.queue_handle, frame).expect("owner frame");
    }

    fn configured(state: &mut WaylandState, floating: FloatingSurface) -> FloatingKind {
        let mut floating = floating;
        floating.configure.mark_configured();
        let kind = popup_kind();
        state.floating_surfaces.insert(kind.clone(), floating);
        kind
    }

    fn committed(state: &mut WaylandState, kind: &FloatingKind) -> Vec<u8> {
        let floating = state.floating_surfaces.get_mut(kind).expect("floating");
        committed_bytes(&mut floating.shm_buffer)
    }

    #[test]
    fn later_configure_commits_latest_blend_not_previous_frame() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        let floating = with_child(fake.floating(Size::new(2, 1), solid(2, 1, [0, 0, 0, 255])));
        let kind = configured(&mut state, floating);
        owner_frame(&fake, &mut state, solid(2, 1, [7, 7, 7, 255]));
        owner_frame(&fake, &mut state, solid(2, 1, BASE));

        fake.configure(&mut state, &kind);

        assert_eq!(committed(&mut state, &kind), BLENDED.to_vec());
    }

    #[test]
    fn configure_between_owner_resize_and_child_frame_commits_blend_not_zeros() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        let floating = with_child(fake.floating(Size::new(1, 1), solid(1, 1, [0, 0, 0, 255])));
        let kind = configured(&mut state, floating);
        owner_frame(&fake, &mut state, solid(2, 1, BASE));

        fake.configure(&mut state, &kind);

        assert_eq!(committed(&mut state, &kind), BLENDED.to_vec());
    }

    #[test]
    fn configure_without_layers_commits_latest_base() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Subsurface);
        let floating = fake.floating(Size::new(2, 1), solid(2, 1, [0, 0, 0, 255]));
        let kind = configured(&mut state, floating);
        owner_frame(&fake, &mut state, solid(2, 1, BASE));

        fake.configure(&mut state, &kind);

        assert_eq!(committed(&mut state, &kind), BASE.repeat(2));
    }

    #[test]
    fn first_configure_commits_preconfigure_blend_once() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        let mut floating = with_child(fake.floating(Size::new(2, 1), solid(2, 1, BASE)));
        repaint(&mut floating, 1);
        let pre = floating.shm_buffer.current_buffer().clone();
        let kind = popup_kind();
        state.floating_surfaces.insert(kind.clone(), floating);

        fake.configure(&mut state, &kind);

        let floating = state.floating_surfaces.get_mut(&kind).expect("floating");
        assert!(floating.configure.is_configured());
        assert_ne!(
            floating.shm_buffer.current_buffer(),
            &pre,
            "swapped exactly once"
        );
        assert_eq!(committed_bytes(&mut floating.shm_buffer), BLENDED.to_vec());
    }

    /// A configure for an `xdg_surface` no floating surface owns is
    /// acknowledged and nothing else.
    #[test]
    fn configure_of_an_unknown_xdg_surface_is_only_acknowledged() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        let kind = popup_kind();
        let floating = fake.floating(Size::new(1, 1), solid(1, 1, BASE));
        state.floating_surfaces.insert(kind.clone(), floating);
        let stranger = fake.xdg_surface();
        fake.requests();
        let configure = xdg_surface::Event::Configure { serial: 3 };
        fake.deliver(&mut state, (&stranger, configure));
        let sent: Vec<(u32, u16, u32)> = fake
            .requests()
            .iter()
            .map(|request| (request.object, request.opcode, request.arg))
            .collect();
        assert_eq!(sent, vec![(id(&stranger), ACK_CONFIGURE, 3)]);
        assert!(!state.floating_surfaces[&kind].configure.is_configured());
    }

    /// The compositor dismissing a popup tears it down.
    #[test]
    fn popup_done_tears_the_popup_down() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        let floating = fake.floating(Size::new(1, 1), solid(1, 1, BASE));
        let popup = floating.xdg_popup.clone();
        state.floating_surfaces.insert(popup_kind(), floating);
        fake.deliver(&mut state, (&popup, xdg_popup::Event::PopupDone));
        assert!(state.floating_surfaces.is_empty());
    }

    /// A dismissal of a popup no floating surface owns, a reposition and a
    /// configure leave every floating surface open.
    #[test]
    fn other_popup_events_keep_the_popup_open() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        let floating = fake.floating(Size::new(1, 1), solid(1, 1, BASE));
        let popup = floating.xdg_popup.clone();
        state.floating_surfaces.insert(popup_kind(), floating);
        let stranger = fake.floating(Size::new(1, 1), solid(1, 1, BASE));
        fake.deliver(
            &mut state,
            (&stranger.xdg_popup, xdg_popup::Event::PopupDone),
        );
        let repositioned = xdg_popup::Event::Repositioned { token: 1 };
        fake.deliver(&mut state, (&popup, repositioned));
        let configure = xdg_popup::Event::Configure {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        };
        fake.deliver(&mut state, (&popup, configure));
        assert!(state.floating_surfaces.contains_key(&popup_kind()));
    }
}
