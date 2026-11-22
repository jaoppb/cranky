//! The Wayland dispatch state: bound globals, outputs, bars, input devices and floating surfaces.
use super::keyboard::KeyboardInput;
use super::pointer_grab::PointerGrab;
use super::types::{FloatingSurface, WaylandBar, WaylandOutputInfo};
use crate::features::layout_engine::domain::{DisplayCommand, FloatingKind};
use crate::shared::env::domain::AppEnvironment;
use crate::shared::events::core::SurfaceKind;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::geometry::{Scale, Size};
use crate::shared::primitives::{ModuleId, MonitorId};
use crate::shared::wayland::adapters::shm::{ShmBuffer, ShmSource};
use crate::shared::wayland::composite::CompositeLayers;
use crate::shared::wayland::composite::route::PointerTarget;
use crate::shared::wayland::ports::DisplayServerError;
use std::collections::HashMap;
use std::sync::Arc;
use wayland_client::QueueHandle;
use wayland_client::protocol::{
    wl_compositor::WlCompositor, wl_pointer::WlPointer, wl_seat::WlSeat, wl_shm::WlShm,
    wl_subcompositor::WlSubcompositor, wl_surface::WlSurface,
};
use wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1;

pub(crate) struct WaylandState {
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
    pub(crate) keyboard: KeyboardInput,

    pub(crate) command_tx: tokio::sync::mpsc::Sender<DisplayCommand>,

    pub(crate) surface_to_id: HashMap<WlSurface, (ModuleId, MonitorId, SurfaceKind)>,
    pub(crate) pointer_surface: Option<WlSurface>,
    pub(crate) pointer_pos: (f64, f64),
    /// Where pointer events currently go: a `surface_to_id` entry, or a child
    /// composited into the floating surface under the pointer.
    pub(crate) pointer_target: Option<PointerTarget>,
    /// Buttons held on `pointer_target`, which stays fixed while any is.
    pub(crate) pointer_grab: PointerGrab,

    pub(crate) floating_surfaces: HashMap<FloatingKind, FloatingSurface>,
    /// Composite child frames that arrived before their floating surface
    /// existed; moved into its `layers` when it is created.
    pub(crate) pending_children: HashMap<FloatingKind, CompositeLayers>,
    pub(crate) last_button_serial: Option<crate::shared::events::core::PointerSerial>,
    pub(crate) app_env: Arc<AppEnvironment>,
}

impl WaylandState {
    /// A state with no global bound, no output and no surface yet, sending
    /// display commands to `command_tx`.
    pub(super) fn new(
        hub: Arc<SignalHub>,
        command_tx: tokio::sync::mpsc::Sender<DisplayCommand>,
        app_env: Arc<AppEnvironment>,
    ) -> Self {
        Self {
            hub,
            compositor: None,
            subcompositor: None,
            layer_shell: None,
            xdg_wm_base: None,
            shm: None,
            outputs: Vec::new(),
            bars: Vec::new(),
            seat: None,
            pointer: None,
            keyboard: KeyboardInput::default(),
            command_tx,
            surface_to_id: HashMap::new(),
            pointer_surface: None,
            pointer_pos: (0.0, 0.0),
            pointer_target: None,
            pointer_grab: PointerGrab::default(),
            floating_surfaces: HashMap::new(),
            pending_children: HashMap::new(),
            last_button_serial: None,
            app_env,
        }
    }

    /// Asks the app for a render, without waiting for it to be queued.
    pub(super) fn request_render(&self) {
        let tx = self.command_tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(DisplayCommand::RequestRender).await;
        });
    }

    /// Withdraws the published scale of monitor `output_name`, if it had one.
    pub(super) fn forget_scale(&self, output_name: &str) {
        let mut scales = self.hub.monitor_scales_rx().borrow().clone();
        if scales.remove(&MonitorId::new(output_name)).is_some() {
            let _ = self.hub.monitor_scales_tx().send(scales);
        }
    }

    /// Publishes `scale` as the scale of monitor `output_name`.
    pub(super) fn publish_scale(&self, output_name: &str, scale: i32) {
        let mut scales = self.hub.monitor_scales_rx().borrow().clone();
        let scale_f32 = i16::try_from(scale).map_or(1.0, f32::from);
        scales.insert(MonitorId::new(output_name), Scale::new(scale_f32));
        let _ = self.hub.monitor_scales_tx().send(scales);
    }
}

/// `global`, or the connection failure naming it as not bound.
pub(super) fn bound_global<'a, T>(
    global: Option<&'a T>,
    name: &str,
) -> Result<&'a T, DisplayServerError> {
    global.ok_or_else(|| DisplayServerError::ConnectionFailed {
        reason: format!("{name} not bound"),
    })
}

/// Two `size` slots from `source`, or the internal error allocating them
/// failed with.
pub(super) fn allocate_buffer(
    source: ShmSource<'_>,
    size: Size,
    queue_handle: &QueueHandle<WaylandState>,
) -> Result<ShmBuffer, DisplayServerError> {
    ShmBuffer::new(source, size, queue_handle)
        .map_err(|error| DisplayServerError::Internal(error.to_string()))
}
