use super::state::WaylandState;
use super::types::WaylandOutputInfo;
use wayland_client::protocol::{
    wl_region::WlRegion,
    wl_registry::{self, WlRegistry},
};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, delegate_noop};

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
            } => {
                let global = AnnouncedGlobal {
                    registry: proxy,
                    name,
                    version,
                    queue_handle,
                };
                on_global(state, &interface, &global);
            }
            wl_registry::Event::GlobalRemove { name } => on_global_remove(state, name),
            _ => {}
        }
    }
}

/// A global the registry announced, with what binding it takes.
struct AnnouncedGlobal<'a> {
    registry: &'a WlRegistry,
    name: u32,
    version: u32,
    queue_handle: &'a QueueHandle<WaylandState>,
}

impl AnnouncedGlobal<'_> {
    /// Binds the global at the version it was announced with.
    fn bind<I>(&self) -> I
    where
        I: Proxy + 'static,
        WaylandState: Dispatch<I, ()>,
    {
        self.bind_at_most(self.version)
    }

    /// Binds the global at its announced version, or `max_version` if lower.
    fn bind_at_most<I>(&self, max_version: u32) -> I
    where
        I: Proxy + 'static,
        WaylandState: Dispatch<I, ()>,
    {
        let version = u32::min(self.version, max_version);
        self.registry
            .bind(self.name, version, self.queue_handle, ())
    }
}

/// Binds `global` if it is one of the interfaces the bar uses; an output is
/// recorded unnamed, at scale 1, until its own events arrive.
fn on_global(state: &mut WaylandState, interface: &str, global: &AnnouncedGlobal<'_>) {
    match interface {
        "wl_compositor" => state.compositor = Some(global.bind()),
        "wl_shm" => state.shm = Some(global.bind()),
        "zwlr_layer_shell_v1" => state.layer_shell = Some(global.bind()),
        "xdg_wm_base" => state.xdg_wm_base = Some(global.bind_at_most(5)),
        "wl_subcompositor" => state.subcompositor = Some(global.bind()),
        "wl_output" => state.outputs.push(WaylandOutputInfo {
            global_id: global.name,
            output: global.bind(),
            name: String::new(),
            scale: 1,
        }),
        "wl_seat" => state.seat = Some(global.bind()),
        _ => {}
    }
}

/// Forgets the output that was global `name`, if it was one: destroys its
/// bar, releases it, withdraws its scale and asks for a render.
fn on_global_remove(state: &mut WaylandState, name: u32) {
    let Some(pos) = state.outputs.iter().position(|info| info.global_id == name) else {
        return;
    };
    let info = state.outputs.remove(pos);
    destroy_bar(state, &info.name);
    info.output.release();
    if !info.name.is_empty() {
        state.forget_scale(&info.name);
    }
    state.request_render();
}

/// Destroys the bar on output `output_name`, if any, with its module
/// surfaces.
fn destroy_bar(state: &mut WaylandState, output_name: &str) {
    let Some(bar_pos) = state
        .bars
        .iter()
        .position(|bar| bar.output_name == output_name)
    else {
        return;
    };
    let mut bar = state.bars.remove(bar_pos);
    for (_, module_surface) in bar.module_surfaces.drain() {
        module_surface.subsurface.destroy();
        module_surface.surface.destroy();
    }
    bar.layer_surface.destroy();
    bar.surface.destroy();
}

#[cfg(test)]
mod tests {
    use crate::features::layout_engine::domain::DisplayCommand;
    use crate::shared::config::domain::PopupEmbed;
    use crate::shared::primitives::geometry::Scale;
    use crate::shared::primitives::{ModuleId, MonitorId};
    use crate::shared::wayland::adapters::wayland::state::WaylandState;
    use crate::shared::wayland::adapters::wayland::tests::fixture::Fake;
    use crate::shared::wayland::adapters::wayland::tests::fixture::wire::id;
    use wayland_client::Proxy;
    use wayland_client::protocol::wl_registry::{self, WlRegistry};

    /// Delivers `event` through the real `wl_registry` dispatch handler.
    fn deliver(fake: &Fake, state: &mut WaylandState, event: wl_registry::Event) {
        let registry: WlRegistry = fake
            .connection()
            .display()
            .get_registry(&fake.queue_handle, ());
        fake.deliver(state, (&registry, event));
    }

    fn remove(fake: &Fake, state: &mut WaylandState, name: u32) {
        deliver(fake, state, wl_registry::Event::GlobalRemove { name });
    }

    /// A state with no global bound, after `announced` were announced.
    fn announced(fake: &Fake, announced: &[(u32, &str, u32)]) -> WaylandState {
        let mut state = fake.state(PopupEmbed::Composite);
        (state.compositor, state.subcompositor, state.layer_shell) = (None, None, None);
        (state.xdg_wm_base, state.shm) = (None, None);
        for &(name, interface, version) in announced {
            let interface = interface.to_string();
            let global = wl_registry::Event::Global {
                name,
                interface,
                version,
            };
            deliver(fake, &mut state, global);
        }
        state
    }

    /// One global of each interface the bar binds, and one it ignores.
    const ANNOUNCED: [(u32, &str, u32); 8] = [
        (1, "wl_compositor", 4),
        (2, "wl_shm", 1),
        (3, "zwlr_layer_shell_v1", 4),
        (4, "xdg_wm_base", 6),
        (5, "wl_subcompositor", 1),
        (6, "wl_output", 4),
        (7, "wl_seat", 7),
        (8, "wp_viewporter", 1),
    ];

    /// Each announced global the bar uses is bound, `xdg_wm_base` at version
    /// 5 at most, each output recorded unnamed at scale 1; the rest are
    /// ignored.
    #[test]
    fn announced_globals_are_bound_by_interface() {
        let fake = Fake::new();
        let state = announced(&fake, &ANNOUNCED);
        let versions = [
            state.compositor.as_ref().map(Proxy::version),
            state.shm.as_ref().map(Proxy::version),
            state.layer_shell.as_ref().map(Proxy::version),
            state.xdg_wm_base.as_ref().map(Proxy::version),
            state.subcompositor.as_ref().map(Proxy::version),
            state.seat.as_ref().map(Proxy::version),
        ];
        assert_eq!(versions, [4, 1, 4, 5, 1, 7].map(Some));
        let outputs: Vec<(u32, &str, i32)> = state
            .outputs
            .iter()
            .map(|info| (info.global_id, info.name.as_str(), info.scale))
            .collect();
        assert_eq!(outputs, vec![(6, "", 1)]);
    }

    /// A state with output 7 on DP-1, its bar with one module surface and its
    /// published scale, and the wire ids of what removing it releases, in
    /// release order.
    fn with_output_bar(fake: &Fake) -> (WaylandState, [u32; 5]) {
        let mut state = fake.state(PopupEmbed::Composite);
        state.outputs.push(fake.output_info(7, "DP-1"));
        let mut bar = fake.bar("DP-1", 1);
        let module = fake.module_surface(&bar.surface);
        let released = [
            id(&module.subsurface),
            id(&module.surface),
            id(&bar.layer_surface),
            id(&bar.surface),
            id(&state.outputs[0].output),
        ];
        bar.module_surfaces.insert(ModuleId::new(2), module);
        state.bars.push(bar);
        let scales = [(MonitorId::new("DP-1"), Scale::new(2.0))].into();
        state.hub.monitor_scales_tx().send(scales).expect("scales");
        (state, released)
    }

    /// Removing an output's global destroys its bar and module surfaces,
    /// releases the output, forgets its scale and asks for a render.
    #[test]
    fn removed_output_loses_its_bar_and_its_scale() {
        crate::test_utils::block_on(async {
            let fake = Fake::new();
            let (mut state, released) = with_output_bar(&fake);
            let (command_tx, mut command_rx) = tokio::sync::mpsc::channel(1);
            state.command_tx = command_tx;
            fake.requests();
            remove(&fake, &mut state, 7);
            assert!(state.outputs.is_empty());
            assert!(state.bars.is_empty());
            assert!(state.hub.monitor_scales_rx().borrow().is_empty());
            let sent: Vec<u32> = fake
                .requests()
                .iter()
                .map(|request| request.object)
                .filter(|object| released.contains(object))
                .collect();
            assert_eq!(sent, released.to_vec());
            let command = command_rx.recv().await;
            assert!(matches!(command, Some(DisplayCommand::RequestRender)));
        });
    }

    /// Removing a global that is no known output changes nothing; removing
    /// an unnamed output leaves the published scales alone.
    #[test]
    fn removed_unknown_or_unnamed_output_keeps_the_scales() {
        crate::test_utils::block_on(async {
            let fake = Fake::new();
            let mut state = fake.state(PopupEmbed::Composite);
            state.outputs.push(fake.output_info(7, ""));
            let scales_rx = state.hub.monitor_scales_rx();
            remove(&fake, &mut state, 9);
            assert_eq!(state.outputs.len(), 1);
            remove(&fake, &mut state, 7);
            assert!(state.outputs.is_empty());
            assert!(!scales_rx.has_changed().expect("hub alive"));
        });
    }

    /// Removing a named output whose scale was never published leaves the
    /// published scales alone.
    #[test]
    fn removed_output_without_a_published_scale_keeps_the_scales() {
        crate::test_utils::block_on(async {
            let fake = Fake::new();
            let mut state = fake.state(PopupEmbed::Composite);
            state.outputs.push(fake.output_info(7, "DP-1"));
            let scales_rx = state.hub.monitor_scales_rx();
            remove(&fake, &mut state, 7);
            assert!(state.outputs.is_empty());
            assert!(!scales_rx.has_changed().expect("hub alive"));
        });
    }
}
