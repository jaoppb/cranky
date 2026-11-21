use super::state::WaylandState;
use super::types::WaylandOutputInfo;
use wayland_client::protocol::{
    wl_output::WlOutput,
    wl_region::WlRegion,
    wl_registry::{self, WlRegistry},
};
use wayland_client::{Connection, Dispatch, QueueHandle, delegate_noop};

delegate_noop!(WaylandState: WlRegion);

impl Dispatch<WlRegistry, ()> for WaylandState {
    fn event(
        state: &mut Self,
        proxy: &WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        queue_handle: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => match interface.as_str() {
                "wl_compositor" => {
                    state.compositor = Some(proxy.bind(name, version, queue_handle, ()));
                }
                "wl_shm" => state.shm = Some(proxy.bind(name, version, queue_handle, ())),
                "zwlr_layer_shell_v1" => {
                    state.layer_shell = Some(proxy.bind(name, version, queue_handle, ()));
                }
                "xdg_wm_base" => {
                    state.xdg_wm_base =
                        Some(proxy.bind(name, u32::min(version, 5), queue_handle, ()));
                }
                "wl_subcompositor" => {
                    state.subcompositor = Some(proxy.bind(name, version, queue_handle, ()));
                }
                "wl_output" => {
                    let output: WlOutput = proxy.bind(name, version, queue_handle, ());
                    state.outputs.push(WaylandOutputInfo {
                        global_id: name,
                        output,
                        name: String::new(),
                        scale: 1,
                    });
                }
                "wl_seat" => state.seat = Some(proxy.bind(name, version, queue_handle, ())),
                _ => {}
            },
            wl_registry::Event::GlobalRemove { name } => {
                if let Some(pos) = state.outputs.iter().position(|info| info.global_id == name) {
                    let info = state.outputs.remove(pos);
                    if let Some(bar_pos) = state
                        .bars
                        .iter()
                        .position(|bar| bar.output_name == info.name)
                    {
                        let mut bar = state.bars.remove(bar_pos);
                        for (_, module_surface) in bar.module_surfaces.drain() {
                            module_surface.subsurface.destroy();
                            module_surface.surface.destroy();
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
