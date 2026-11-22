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
    use crate::features::layout_engine::domain::DisplayCommand;
    use crate::shared::config::domain::PopupEmbed;
    use crate::shared::primitives::MonitorId;
    use crate::shared::primitives::geometry::{Scale, Size};
    use crate::shared::wayland::adapters::wayland::state::WaylandState;
    use crate::shared::wayland::adapters::wayland::tests::fixture::wire::id;
    use crate::shared::wayland::adapters::wayland::tests::fixture::{Fake, lose_runtime_dir};
    use crate::shared::wayland::adapters::wayland::types::ConfigureGate;
    use wayland_client::protocol::wl_output::{self, WlOutput};
    use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1;

    /// `zwlr_layer_surface_v1.ack_configure`.
    const ACK_CONFIGURE: u16 = 6;

    fn published_scale(state: &WaylandState, name: &str) -> Option<Scale> {
        state
            .hub
            .monitor_scales_rx()
            .borrow()
            .get(&MonitorId::new(name))
            .copied()
    }

    /// A state with a bar on DP-1 and an unnamed output, and that output.
    fn with_unnamed_output(fake: &Fake) -> (WaylandState, WlOutput) {
        let mut state = fake.state(PopupEmbed::Composite);
        state.bars.push(fake.bar("DP-1", 1));
        state.outputs.push(fake.output_info(7, ""));
        let output = state.outputs[0].output.clone();
        (state, output)
    }

    /// An output's scale is published, and given to its bar, only once the
    /// output has a name; every later scale follows.
    #[test]
    fn output_scale_is_published_once_the_output_is_named() {
        let fake = Fake::new();
        let (mut state, output) = with_unnamed_output(&fake);
        fake.deliver(&mut state, (&output, wl_output::Event::Scale { factor: 2 }));
        assert_eq!(state.outputs[0].scale, 2);
        assert_eq!(published_scale(&state, "DP-1"), None);
        assert_eq!(state.bars[0].scale, 1);
        let name = "DP-1".to_string();
        fake.deliver(&mut state, (&output, wl_output::Event::Name { name }));
        assert_eq!(published_scale(&state, "DP-1"), Some(Scale::new(2.0)));
        assert_eq!(state.bars[0].scale, 2);
        fake.deliver(&mut state, (&output, wl_output::Event::Scale { factor: 3 }));
        fake.deliver(&mut state, (&output, wl_output::Event::Done));
        assert_eq!(published_scale(&state, "DP-1"), Some(Scale::new(3.0)));
        assert_eq!(state.bars[0].scale, 3);
    }

    /// An empty name publishes nothing, and events for an output the state
    /// does not know change nothing.
    #[test]
    fn unnamed_or_unknown_output_publishes_no_scale() {
        let fake = Fake::new();
        let (mut state, output) = with_unnamed_output(&fake);
        let name = String::new();
        fake.deliver(&mut state, (&output, wl_output::Event::Name { name }));
        let unknown = fake.output_info(8, "").output;
        fake.deliver(
            &mut state,
            (&unknown, wl_output::Event::Scale { factor: 2 }),
        );
        assert!(state.hub.monitor_scales_rx().borrow().is_empty());
        assert_eq!(state.outputs[0].scale, 1);
    }

    /// A configure to a new size resizes the bar, gives it a buffer of that
    /// size and asks for a render.
    #[test]
    fn layer_surface_configure_to_a_new_size_reallocates_and_requests_a_render() {
        crate::test_utils::block_on(async {
            let fake = Fake::new();
            let mut state = fake.state(PopupEmbed::Composite);
            let (command_tx, mut command_rx) = tokio::sync::mpsc::channel(1);
            state.command_tx = command_tx;
            state.bars.push(fake.bar("DP-1", 1));
            fake.configure_bar(&mut state, Size::new(3, 2));
            let bar = &state.bars[0];
            assert_eq!((bar.width, bar.height), (3, 2));
            assert_eq!((bar.shm_buffer.width(), bar.shm_buffer.height()), (3, 2));
            let command = command_rx.recv().await;
            assert!(matches!(command, Some(DisplayCommand::RequestRender)));
        });
    }

    /// A resize whose buffer cannot be allocated keeps the old buffer but
    /// records the new size.
    #[test]
    fn layer_surface_resize_without_memory_keeps_the_old_buffer() {
        crate::test_utils::block_on(async {
            let fake = Fake::new();
            let mut state = fake.state(PopupEmbed::Composite);
            state.bars.push(fake.bar("DP-1", 1));
            lose_runtime_dir(&mut state);
            fake.configure_bar(&mut state, Size::new(3, 2));
            let bar = &state.bars[0];
            assert_eq!((bar.width, bar.height), (3, 2));
            assert_eq!((bar.shm_buffer.width(), bar.shm_buffer.height()), (1, 1));
        });
    }

    /// A configure at the bar's own size keeps its buffer.
    #[test]
    fn layer_surface_configure_at_the_same_size_keeps_the_buffer() {
        crate::test_utils::block_on(async {
            let fake = Fake::new();
            let mut state = fake.state(PopupEmbed::Composite);
            state.bars.push(fake.bar("DP-1", 1));
            let buffer = state.bars[0].shm_buffer.current_buffer().clone();
            fake.configure_bar(&mut state, Size::new(1, 1));
            assert_eq!(state.bars[0].shm_buffer.current_buffer(), &buffer);
        });
    }

    /// A configure for a layer surface no bar owns is acknowledged and
    /// nothing else.
    #[test]
    fn layer_surface_configure_of_no_bar_is_only_acknowledged() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        let mut bar = fake.bar("DP-1", 1);
        bar.configure = ConfigureGate::Pending;
        state.bars.push(bar);
        let stranger_bar = fake.bar("HDMI-A-1", 1);
        let stranger = stranger_bar.layer_surface.clone();
        fake.requests();
        let configure = zwlr_layer_surface_v1::Event::Configure {
            serial: 4,
            width: 3,
            height: 2,
        };
        fake.deliver(&mut state, (&stranger, configure));
        let acks: Vec<(u32, u32)> = fake
            .requests()
            .iter()
            .filter(|request| request.opcode == ACK_CONFIGURE)
            .map(|request| (request.object, request.arg))
            .collect();
        assert_eq!(acks, vec![(id(&stranger), 4)]);
        assert_eq!(state.bars[0].configure, ConfigureGate::Pending);
    }

    /// A `closed` event changes nothing.
    #[test]
    fn layer_surface_closed_changes_nothing() {
        let fake = Fake::new();
        let mut state = fake.state(PopupEmbed::Composite);
        let mut bar = fake.bar("DP-1", 1);
        bar.configure = ConfigureGate::Pending;
        let layer_surface = bar.layer_surface.clone();
        state.bars.push(bar);
        fake.deliver(
            &mut state,
            (&layer_surface, zwlr_layer_surface_v1::Event::Closed),
        );
        assert_eq!(state.bars[0].configure, ConfigureGate::Pending);
        assert_eq!((state.bars[0].width, state.bars[0].height), (1, 1));
    }

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
        fake.configure_bar(&mut state, Size::new(0, 0));
        assert_eq!(state.bars[0].configure, ConfigureGate::Configured);
        assert_eq!((state.bars[0].width, state.bars[0].height), (1, 1));
    }

    /// A resize with no `wl_shm` bound keeps the old buffer but records the
    /// new size and asks for a render.
    #[test]
    fn layer_surface_resize_without_shm_keeps_the_old_buffer() {
        crate::test_utils::block_on(async {
            let fake = Fake::new();
            let mut state = fake.state(PopupEmbed::Composite);
            let (command_tx, mut command_rx) = tokio::sync::mpsc::channel(1);
            state.command_tx = command_tx;
            state.shm = None;
            state.bars.push(fake.bar("DP-1", 1));
            fake.configure_bar(&mut state, Size::new(3, 2));
            let bar = &state.bars[0];
            assert_eq!((bar.width, bar.height), (3, 2));
            assert_eq!((bar.shm_buffer.width(), bar.shm_buffer.height()), (1, 1));
            let command = command_rx.recv().await;
            assert!(matches!(command, Some(DisplayCommand::RequestRender)));
        });
    }
}
