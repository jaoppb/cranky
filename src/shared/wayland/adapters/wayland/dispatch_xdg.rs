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
                tracing::debug!("Attaching buffer to floating surface");
                floating
                    .surface
                    .attach(Some(floating.shm_buffer.current_buffer()), 0, 0);
                let width = i32::try_from(floating.size.width()).unwrap_or(0);
                let height = i32::try_from(floating.size.height()).unwrap_or(0);
                floating.surface.damage_buffer(0, 0, width, height);
                floating.surface.commit();
                floating.shm_buffer.swap_buffers();
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
