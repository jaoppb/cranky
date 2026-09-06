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
        _data: &(),
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
        _data: &(),
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
                #[allow(clippy::as_conversions)]
                floating.surface.damage_buffer(
                    0,
                    0,
                    floating.size.width() as i32,
                    floating.size.height() as i32,
                );
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
        _data: &(),
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
                if let Some(kind) = matching_kind
                    && let Some(floating) = state.floating_surfaces.remove(&kind)
                {
                    state.surface_to_id.remove(&floating.surface);
                    if let crate::features::layout_engine::domain::FloatingKind::Popup(target) =
                        kind
                    {
                        let _ = state.hub.pointer_tx().send((
                            target.module_id(),
                            target.monitor_id().clone(),
                            crate::shared::events::core::PointerEvent::PopupDismissed,
                        ));
                    }
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
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}
