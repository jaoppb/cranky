//! A bar's layer surface: which outputs get one, how it is opened, and the
//! geometry it reserves on its output.
use super::state::{WaylandState, allocate_buffer, bound_global};
use super::types::{ConfigureGate, WaylandBar};
use crate::shared::config::domain::MarginConfig;
use crate::shared::primitives::geometry::Size;
use crate::shared::wayland::adapters::shm::ShmSource;
use crate::shared::wayland::ports::DisplayServerError;
use std::collections::HashMap;
use tracing::debug;
use wayland_client::QueueHandle;
use wayland_client::protocol::{
    wl_compositor::WlCompositor, wl_output::WlOutput, wl_surface::WlSurface,
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{Anchor, ZwlrLayerSurfaceV1},
};

/// The width a new bar's buffer starts at, until its first configure.
const INITIAL_WIDTH: u32 = 1920;

/// The bar an output is about to get: the output, its name and scale, and
/// the configured height and margin.
struct BarSpec<'a> {
    output: &'a WlOutput,
    name: String,
    scale: i32,
    height: u32,
    margin: MarginConfig,
}

/// The globals a bar is opened from.
struct BarGlobals<'a> {
    compositor: &'a WlCompositor,
    layer_shell: &'a ZwlrLayerShellV1,
    shm: ShmSource<'a>,
}

impl WaylandState {
    pub(crate) fn create_bar(
        &mut self,
        output: &WlOutput,
        queue_handle: &QueueHandle<Self>,
    ) -> Result<(), DisplayServerError> {
        let Some(spec) = self.bar_spec(output)? else {
            return Ok(());
        };
        debug!(
            "Creating bar for output: {} (height: {}, scale: {})",
            spec.name, spec.height, spec.scale
        );
        let bar = self.bar_globals()?.open_bar(&spec, queue_handle)?;
        self.bars.push(bar);
        self.publish_scale(&spec.name, spec.scale);
        Ok(())
    }

    /// Every named output without a bar.
    pub(super) fn outputs_without_bars(&self) -> Vec<WlOutput> {
        self.outputs
            .iter()
            .filter(|info| !info.name.is_empty())
            .filter(|info| !self.bars.iter().any(|bar| bar.output_name == info.name))
            .map(|info| info.output.clone())
            .collect()
    }

    /// The bar `output` should get, or `None` while it has no name or once
    /// it has a bar.
    fn bar_spec<'a>(
        &self,
        output: &'a WlOutput,
    ) -> Result<Option<BarSpec<'a>>, DisplayServerError> {
        let info = self
            .outputs
            .iter()
            .find(|info| &info.output == output)
            .ok_or_else(|| DisplayServerError::ConnectionFailed {
                reason: "Output not found".to_string(),
            })?;
        if info.name.is_empty() || self.bars.iter().any(|bar| bar.output_name == info.name) {
            return Ok(None);
        }
        let root_config = self.hub.config_rx().borrow().root().clone();
        Ok(Some(BarSpec {
            output,
            name: info.name.clone(),
            scale: info.scale,
            height: root_config.height().value(),
            margin: *root_config.margin(),
        }))
    }

    fn bar_globals(&self) -> Result<BarGlobals<'_>, DisplayServerError> {
        Ok(BarGlobals {
            compositor: bound_global(self.compositor.as_ref(), "Compositor")?,
            layer_shell: bound_global(self.layer_shell.as_ref(), "Layer shell")?,
            shm: ShmSource::new(
                bound_global(self.shm.as_ref(), "SHM")?,
                self.app_env.xdg_runtime_dir().as_path(),
            ),
        })
    }
}

impl BarGlobals<'_> {
    /// Opens `spec`'s bar, not configured yet.
    fn open_bar(
        &self,
        spec: &BarSpec<'_>,
        queue_handle: &QueueHandle<WaylandState>,
    ) -> Result<WaylandBar, DisplayServerError> {
        let (surface, layer_surface) = self.open_layer_surface(spec, queue_handle);
        let size = Size::new(INITIAL_WIDTH, spec.height);
        let shm_buffer = allocate_buffer(self.shm, size, queue_handle)?;
        Ok(WaylandBar {
            output_name: spec.name.clone(),
            surface,
            layer_surface,
            shm_buffer,
            width: INITIAL_WIDTH,
            height: spec.height,
            config_height: spec.height,
            config_margin: spec.margin,
            scale: spec.scale,
            module_surfaces: HashMap::new(),
            configure: ConfigureGate::Pending,
        })
    }

    /// A surface at `spec`'s scale with a top-layer surface anchored to the
    /// top edge of `spec`'s output, sized and committed.
    fn open_layer_surface(
        &self,
        spec: &BarSpec<'_>,
        queue_handle: &QueueHandle<WaylandState>,
    ) -> (WlSurface, ZwlrLayerSurfaceV1) {
        let surface = self.compositor.create_surface(queue_handle, ());
        surface.set_buffer_scale(spec.scale);
        let layer_surface = self.layer_shell.get_layer_surface(
            &surface,
            Some(spec.output),
            Layer::Top,
            "cranky".to_string(),
            queue_handle,
            (),
        );
        layer_surface.set_anchor(Anchor::Top | Anchor::Left | Anchor::Right);
        apply_bar_geometry(&layer_surface, spec.height, &spec.margin);
        surface.commit();
        (surface, layer_surface)
    }
}

/// Sizes `layer_surface` to `height` with `margin` around it, and reserves
/// both as its exclusive zone.
pub(super) fn apply_bar_geometry(
    layer_surface: &ZwlrLayerSurfaceV1,
    height: u32,
    margin: &MarginConfig,
) {
    layer_surface.set_size(0, height);
    layer_surface.set_margin(
        margin.top().value(),
        margin.right().value(),
        margin.bottom().value(),
        margin.left().value(),
    );
    let height_i32 = i32::try_from(height).unwrap_or(0);
    let zone = height_i32
        .saturating_add(margin.top().value())
        .saturating_add(margin.bottom().value());
    layer_surface.set_exclusive_zone(zone);
}
