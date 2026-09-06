use super::state::WaylandState;
use crate::shared::wayland::adapters::shm::ShmBuffer;
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
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        let mut scale_updated_output = None;
        if let Some(info) = state.outputs.iter_mut().find(|i| &i.output == proxy) {
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
            if let Some(bar) = state.bars.iter_mut().find(|b| b.output_name == name) {
                bar.scale = factor;
                bar.surface.set_buffer_scale(factor);
            }
            let mut scales = state.hub.monitor_scales_rx().borrow().clone();
            scales.insert(
                crate::shared::primitives::MonitorId::new(&name),
                #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
                crate::shared::primitives::geometry::Scale::new(factor as f32),
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
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let zwlr_layer_surface_v1::Event::Configure {
            serial,
            width,
            height,
        } = event
        {
            proxy.ack_configure(serial);
            if let Some(bar) = state.bars.iter_mut().find(|b| &b.layer_surface == proxy) {
                bar.configured = true;
                if width > 0 && height > 0 {
                    let old_width = bar.width;
                    let old_height = bar.height;

                    bar.width = width;
                    bar.height = height;

                    if old_width != width || old_height != height {
                        debug!("Bar resized to {}x{}", width, height);

                        if let Ok(new_shm) = ShmBuffer::new(
                            state.shm.as_ref().expect("SHM bound"),
                            width,
                            height,
                            qh,
                            state.app_env.xdg_runtime_dir().as_path(),
                        ) {
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
