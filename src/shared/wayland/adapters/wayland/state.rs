use super::types::{FloatingSurface, WaylandBar, WaylandOutputInfo};
use crate::features::layout_engine::domain::{DisplayCommand, FloatingKind};
use crate::shared::env::domain::AppEnvironment;
use crate::shared::events::core::SurfaceKind;
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

    pub(crate) surface_to_id: HashMap<WlSurface, (ModuleId, MonitorId, SurfaceKind)>,
    pub(crate) pointer_surface: Option<WlSurface>,
    pub(crate) pointer_pos: (f64, f64),

    pub(crate) font_system: FontSystem,
    pub(crate) swash_cache: SwashCache,
    pub(crate) floating_surfaces: HashMap<FloatingKind, FloatingSurface>,
    pub(crate) last_button_serial: Option<crate::shared::events::core::PointerSerial>,
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
        let height_i32 = i32::try_from(bar_height.value()).unwrap_or(0);
        let zone = height_i32
            .saturating_add(margin.top().value())
            .saturating_add(margin.bottom().value());
        layer_surface.set_exclusive_zone(zone);
        surface.commit();

        let shm_buffer = ShmBuffer::new(
            shm,
            1920,
            bar_height.value(),
            qh,
            self.app_env.xdg_runtime_dir().as_path(),
        )
        .map_err(|e| DisplayServerError::Internal(e.to_string()))?;

        self.bars.push(WaylandBar {
            output_name: output_name.clone(),
            surface,
            layer_surface,
            shm_buffer,
            width: 1920,
            height: bar_height.value(),
            config_height: bar_height.value(),
            config_margin: *margin,
            scale: output_scale,
            module_surfaces: HashMap::new(),
            configured: false,
        });

        let mut scales = self.hub.monitor_scales_rx().borrow().clone();
        let scale_f32 = i16::try_from(output_scale).map_or(1.0, f32::from);
        scales.insert(
            MonitorId::new(&output_name),
            crate::shared::primitives::geometry::Scale::new(scale_f32),
        );
        let _ = self.hub.monitor_scales_tx().send(scales);

        Ok(())
    }
}
