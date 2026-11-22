use super::state::WaylandState;
use crate::shared::primitives::geometry::Size;
use crate::shared::wayland::adapters::shm::{ShmBuffer, ShmSource};
use tracing::debug;
use wayland_client::protocol::wl_output::{self, WlOutput};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::{
    self, ZwlrLayerSurfaceV1,
};

impl Dispatch<WlOutput, ()> for WaylandState {
    fn event(
        state: &mut Self,
        proxy: &WlOutput,
        event: wl_output::Event,
        (): &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        let mut scale_updated_output = None;
        if let Some(info) = state.outputs.iter_mut().find(|info| &info.output == proxy) {
            match event {
                wl_output::Event::Name { name } => {
                    info.name = name;
                    if !info.name.is_empty() {
                        scale_updated_output = Some((info.name.clone(), info.scale));
                    }
                }
                wl_output::Event::Scale { factor } => {
                    info.scale = factor;
                    if !info.name.is_empty() {
                        scale_updated_output = Some((info.name.clone(), factor));
                    }
                }
                _ => {}
            }
        }
        if let Some((name, factor)) = scale_updated_output {
            if let Some(bar) = state.bars.iter_mut().find(|bar| bar.output_name == name) {
                bar.scale = factor;
                bar.surface.set_buffer_scale(factor);
            }
            let mut scales = state.hub.monitor_scales_rx().borrow().clone();
            let factor_f32 = i16::try_from(factor).map_or(1.0, f32::from);
            scales.insert(
                crate::shared::primitives::MonitorId::new(&name),
                crate::shared::primitives::geometry::Scale::new(factor_f32),
            );
            let _ = state.hub.monitor_scales_tx().send(scales);
        }
    }
}

impl Dispatch<ZwlrLayerSurfaceV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        proxy: &ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        (): &(),
        _conn: &Connection,
        queue_handle: &QueueHandle<Self>,
    ) {
        if let zwlr_layer_surface_v1::Event::Configure {
            serial,
            width,
            height,
        } = event
        {
            proxy.ack_configure(serial);
            if let Some(bar) = state
                .bars
                .iter_mut()
                .find(|bar| &bar.layer_surface == proxy)
            {
                bar.configure.mark_configured();
                if width > 0 && height > 0 {
                    let old_width = bar.width;
                    let old_height = bar.height;

                    bar.width = width;
                    bar.height = height;

                    if old_width != width || old_height != height {
                        debug!("Bar resized to {}x{}", width, height);

                        if let Some(shm) = state.shm.as_ref()
                            && let Ok(new_shm) = ShmBuffer::new(
                                ShmSource::new(shm, state.app_env.xdg_runtime_dir().as_path()),
                                Size::new(width, height),
                                queue_handle,
                            )
                        {
                            bar.shm_buffer = new_shm;
                        }
                    }

                    let tx = state.command_tx.clone();
                    tokio::spawn(async move {
                        let _ = tx
                            .send(crate::features::layout_engine::domain::DisplayCommand::RequestRender)
                            .await;
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::shared::config::domain::PopupEmbed;
    use crate::shared::wayland::adapters::wayland::tests::fixture::Fake;
    use crate::shared::wayland::adapters::wayland::types::ConfigureGate;

    /// The layer surface's first `configure` opens the bar's configure gate.
    /// A zero size leaves the bar's size to the client, so no resize (and no
    /// render request, which needs a runtime) follows.
    #[test]
    fn layer_surface_configure_marks_the_bar_configured() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        let mut bar = fake.bar("DP-1", 1);
        bar.configure = ConfigureGate::Pending;
        state.bars.push(bar);
        fake.configure_bar(&mut state, 0, 0, 0);
        assert_eq!(state.bars[0].configure, ConfigureGate::Configured);
        assert_eq!((state.bars[0].width, state.bars[0].height), (1, 1));
    }
}
