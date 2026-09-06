use super::types::{FloatingSurface, WaylandBar, WaylandOutputInfo};
use crate::features::layout_engine::domain::{DisplayCommand, FloatingKind};
use crate::shared::env::domain::AppEnvironment;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::{ModuleId, MonitorId};
use crate::shared::wayland::adapters::shm::ShmBuffer;
use crate::shared::wayland::ports::DisplayServerError;
use cosmic_text::{FontSystem, SwashCache};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;
use wayland_client::protocol::{
    wl_compositor::WlCompositor, wl_output::WlOutput, wl_pointer::WlPointer, wl_seat::WlSeat,
    wl_shm::WlShm, wl_subcompositor::WlSubcompositor, wl_surface::WlSurface,
};
use wayland_client::QueueHandle;
use wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase;
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::Anchor,
};

pub struct WaylandState {
    pub(crate) hub: Arc<SignalHub>,
    pub(crate) compositor: Option<WlCompositor>,
    pub(crate) subcompositor: Option<WlSubcompositor>,
    pub(crate) layer_shell: Option<ZwlrLayerShellV1>,
    pub(crate) xdg_wm_base: Option<XdgWmBase>,
    pub(crate) shm: Option<WlShm>,
    pub(crate) outputs: Vec<WaylandOutputInfo>,
    pub(crate) bars: Vec<WaylandBar>,
    pub(crate) seat: Option<WlSeat>,
    pub(crate) pointer: Option<WlPointer>,

    pub(crate) command_tx: tokio::sync::mpsc::Sender<DisplayCommand>,

    pub(crate) surface_to_id: HashMap<WlSurface, (ModuleId, MonitorId)>,
    pub(crate) pointer_surface: Option<WlSurface>,
    pub(crate) pointer_pos: (f64, f64),

    pub(crate) font_system: FontSystem,
    pub(crate) swash_cache: SwashCache,
    pub(crate) floating_surfaces: HashMap<FloatingKind, FloatingSurface>,
    pub(crate) app_env: Arc<AppEnvironment>,
}

impl WaylandState {
    pub(crate) fn create_bar(
        &mut self,
        output: &WlOutput,
        qh: &QueueHandle<Self>,
    ) -> Result<(), DisplayServerError> {
        let (output_name, output_scale) = {
            let info = self
                .outputs
                .iter()
                .find(|i| &i.output == output)
                .ok_or_else(|| DisplayServerError::ConnectionFailed {
                    reason: "Output not found".to_string(),
                })?;
            if info.name.is_empty() {
                return Ok(());
            }
            (info.name.clone(), info.scale)
        };

        if self.bars.iter().any(|b| b.output_name == output_name) {
            return Ok(());
        }

        let root_config = self.hub.config_rx().borrow().root().clone();
        let bar_height = root_config.height();
        let margin = root_config.margin();
        debug!(
            "Creating bar for output: {} (height: {}, scale: {})",
            output_name,
            bar_height.value(),
            output_scale
        );

        let compositor = self
            .compositor
            .as_ref()
            .ok_or_else(|| DisplayServerError::ConnectionFailed {
                reason: "Compositor not bound".to_string(),
            })?;
        let layer_shell =
            self.layer_shell
                .as_ref()
                .ok_or_else(|| DisplayServerError::ConnectionFailed {
                    reason: "Layer shell not bound".to_string(),
                })?;
        let shm = self
            .shm
            .as_ref()
            .ok_or_else(|| DisplayServerError::ConnectionFailed {
                reason: "SHM not bound".to_string(),
            })?;

        let surface = compositor.create_surface(qh, ());
        surface.set_buffer_scale(output_scale);
        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            Some(output),
            Layer::Top,
            "cranky".to_string(),
            qh,
            (),
        );

        layer_surface.set_anchor(Anchor::Top | Anchor::Left | Anchor::Right);
        layer_surface.set_size(0, bar_height.value());
        layer_surface.set_margin(
            margin.top().value(),
            margin.right().value(),
            margin.bottom().value(),
            margin.left().value(),
        );
        #[allow(clippy::as_conversions)]
        layer_surface.set_exclusive_zone(
            bar_height.value() as i32 + margin.top().value() + margin.bottom().value(),
        );
        surface.commit();

        let shm_buffer = ShmBuffer::new(
            shm,
            1920,
            bar_height.value(),
            qh,
            self.app_env.xdg_runtime_dir().as_path(),
        )
        .expect("Failed to create SHM buffer");

        self.bars.push(WaylandBar {
            output_name: output_name.clone(),
            surface,
            layer_surface,
            shm_buffer,
            width: 1920,
            height: bar_height.value(),
            config_height: bar_height.value(),
            config_margin: margin.clone(),
            scale: output_scale,
            module_surfaces: HashMap::new(),
            configured: false,
        });

        let mut scales = self.hub.monitor_scales_rx().borrow().clone();
        scales.insert(
            MonitorId::new(&output_name),
            #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
            crate::shared::primitives::geometry::Scale::new(output_scale as f32),
        );
        let _ = self.hub.monitor_scales_tx().send(scales);

        Ok(())
    }
}
