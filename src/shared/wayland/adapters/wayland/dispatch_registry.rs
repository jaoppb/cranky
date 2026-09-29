use super::state::WaylandState;
use super::types::WaylandOutputInfo;
use crate::shared::wayland::adapters::shm::BufferUserData;
use wayland_client::protocol::{
    wl_buffer::{self, WlBuffer},
    wl_compositor::{self, WlCompositor},
    wl_output::WlOutput,
    wl_registry::{self, WlRegistry},
    wl_shm::{self, WlShm},
    wl_shm_pool::{self, WlShmPool},
    wl_subcompositor::{self, WlSubcompositor},
    wl_subsurface::{self, WlSubsurface},
    wl_surface::{self, WlSurface},
};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::{self, ZwlrLayerShellV1};

impl Dispatch<WlRegistry, ()> for WaylandState {
    fn event(
        state: &mut Self,
        proxy: &WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => match interface.as_str() {
                "wl_compositor" => state.compositor = Some(proxy.bind(name, version, qh, ())),
                "wl_shm" => state.shm = Some(proxy.bind(name, version, qh, ())),
                "zwlr_layer_shell_v1" => {
                    state.layer_shell = Some(proxy.bind(name, version, qh, ()));
                }
                "xdg_wm_base" => {
                    state.xdg_wm_base = Some(proxy.bind(name, u32::min(version, 5), qh, ()));
                }
                "wl_subcompositor" => state.subcompositor = Some(proxy.bind(name, version, qh, ())),
                "wl_output" => {
                    let output: WlOutput = proxy.bind(name, version, qh, ());
                    state.outputs.push(WaylandOutputInfo {
                        global_id: name,
                        output,
                        name: String::new(),
                        scale: 1,
                    });
                }
                "wl_seat" => state.seat = Some(proxy.bind(name, version, qh, ())),
                _ => {}
            },
            wl_registry::Event::GlobalRemove { name } => {
                if let Some(pos) = state.outputs.iter().position(|o| o.global_id == name) {
                    let info = state.outputs.remove(pos);
                    if let Some(bar_pos) =
                        state.bars.iter().position(|b| b.output_name == info.name)
                    {
                        let mut bar = state.bars.remove(bar_pos);
                        for (_, ms) in bar.module_surfaces.drain() {
                            ms.subsurface.destroy();
                            ms.surface.destroy();
                        }
                        bar.layer_surface.destroy();
                        bar.surface.destroy();
                    }
                    info.output.release();

                    if !info.name.is_empty() {
                        let mut scales = state.hub.monitor_scales_rx().borrow().clone();
                        if scales
                            .remove(&crate::shared::primitives::MonitorId::new(&info.name))
                            .is_some()
                        {
                            let _ = state.hub.monitor_scales_tx().send(scales);
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
            _ => {}
        }
    }
}

impl Dispatch<WlCompositor, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlCompositor,
        _: wl_compositor::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlShm, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlShm,
        _: wl_shm::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrLayerShellV1, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &ZwlrLayerShellV1,
        _: zwlr_layer_shell_v1::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlSubcompositor, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlSubcompositor,
        _: wl_subcompositor::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlSubsurface, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlSubsurface,
        _: wl_subsurface::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlSurface, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlSurface,
        _: wl_surface::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlBuffer, BufferUserData> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlBuffer,
        event: wl_buffer::Event,
        data: &BufferUserData,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if matches!(event, wl_buffer::Event::Release) {
            data.set_busy(false);
            tracing::trace!("WlBuffer released by compositor");
        }
    }
}

impl Dispatch<WlShmPool, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlShmPool,
        _: wl_shm_pool::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
