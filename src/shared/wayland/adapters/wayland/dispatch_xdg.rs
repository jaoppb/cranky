use super::state::WaylandState;
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
            tracing::debug!("xdg_surface Configure serial={serial}");
            proxy.ack_configure(serial);
            if let Some(floating) = state
                .floating_surfaces
                .values_mut()
                .find(|f| &f.xdg_surface == proxy)
            {
                // Re-blend rather than attach the back slot as-is: after any
                // commit it holds the previous frame, after a resize zeros.
                tracing::debug!("Repainting floating surface on configure");
                floating.configure.mark_configured();
                let scale = floating.scale;
                super::floating_composite::repaint(floating, scale);
            } else {
                tracing::debug!("Configure event for unknown xdg_surface");
            }
        }
    }
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
            xdg_popup::Event::Repositioned { token } => {
                tracing::debug!("Floating surface popup repositioned (token={token})");
            }
            xdg_popup::Event::PopupDone => {
                tracing::debug!("xdg_popup Event: PopupDone");
                let matching_kind = state
                    .floating_surfaces
                    .iter()
                    .find(|(_, f)| &f.xdg_popup == proxy)
                    .map(|(k, _)| k.clone());
                if let Some(kind) = matching_kind {
                    super::floating_teardown::teardown_floating(state, &kind, true);
                }
            }
            other => tracing::debug!("xdg_popup Event: {other:?}"),
        }
    }
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
    use crate::shared::config::domain::PopupEmbed;
    use crate::shared::primitives::render::RenderBuffer;
    use crate::shared::primitives::{ModuleId, MonitorId, PopupOffset, Position, Size};
    use crate::shared::wayland::adapters::wayland::floating_anchor::{
        AnchorInfo, FloatingParentRole,
    };
    use crate::shared::wayland::adapters::wayland::floating_composite::repaint;
    use crate::shared::wayland::adapters::wayland::floating_setup::{
        FloatingPlacement, update_existing_floating,
    };
    use crate::shared::wayland::adapters::wayland::state::WaylandState;
    use crate::shared::wayland::adapters::wayland::tests::fixture::{
        Fake, committed_bytes, popup_kind, solid,
    };
    use crate::shared::wayland::adapters::wayland::types::FloatingSurface;
    use crate::features::layout_engine::domain::FloatingKind;

    const BASE: [u8; 4] = [5, 5, 5, 255];
    /// A 2×1 `BASE` with the 1×1 child at logical (1, 0).
    const BLENDED: [u8; 8] = [5, 5, 5, 255, 1, 2, 3, 255];

    fn with_child(mut floating: FloatingSurface) -> FloatingSurface {
        floating.layers.upsert(
            ModuleId::new(9),
            MonitorId::new("DP-1"),
            Position::new(1, 0),
            solid(1, 1, [1, 2, 3, 255]),
        );
        floating
    }

    /// An owner frame `base` placed at `base`'s own size.
    fn owner_frame(fake: &Fake, state: &mut WaylandState, kind: &FloatingKind, base: &RenderBuffer) {
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
            size: *base.size(),
            text_w: 2,
            text_h: 1,
            offset: PopupOffset::default(),
        };
        update_existing_floating(state, &fake.qh, kind, base, &anchor, &placement)
            .expect("owner frame");
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
        owner_frame(&fake, &mut state, &kind, &solid(2, 1, [7, 7, 7, 255]));
        owner_frame(&fake, &mut state, &kind, &solid(2, 1, BASE));

        fake.configure(&mut state, &kind);

        assert_eq!(committed(&mut state, &kind), BLENDED.to_vec());
    }

    #[test]
    fn configure_between_owner_resize_and_child_frame_commits_blend_not_zeros() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        let floating = with_child(fake.floating(Size::new(1, 1), solid(1, 1, [0, 0, 0, 255])));
        let kind = configured(&mut state, floating);
        owner_frame(&fake, &mut state, &kind, &solid(2, 1, BASE));

        fake.configure(&mut state, &kind);

        assert_eq!(committed(&mut state, &kind), BLENDED.to_vec());
    }

    #[test]
    fn configure_without_layers_commits_latest_base() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Subsurface);
        let floating = fake.floating(Size::new(2, 1), solid(2, 1, [0, 0, 0, 255]));
        let kind = configured(&mut state, floating);
        owner_frame(&fake, &mut state, &kind, &solid(2, 1, BASE));

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
        assert_ne!(floating.shm_buffer.current_buffer(), &pre, "swapped exactly once");
        assert_eq!(committed_bytes(&mut floating.shm_buffer), BLENDED.to_vec());
    }
}
