//! A compositor-less `WaylandState` for unit tests: protocol objects are
//! bound over a socket pair nobody reads from, so requests are just queued.

use crate::features::layout_engine::domain::{FloatingKind, PopupTarget};
use crate::features::metrics::domain::MetricsConfig;
use crate::shared::config::domain::{
    Config, MarginConfig, ModulesConfig, PopupConfig, PopupEmbed, RenderingMode, RootConfig,
    TooltipConfig,
};
use crate::shared::env::domain::{AppEnvironment, HomeDir, RustLog, XdgCacheHome, XdgRuntimeDir};
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::primitives::{ModuleId, MonitorId};
use crate::shared::wayland::adapters::shm::ShmBuffer;
use crate::shared::wayland::adapters::wayland::state::WaylandState;
use crate::shared::wayland::adapters::wayland::types::{FloatingSurface, WaylandBar};
use crate::shared::wayland::composite::CompositeLayers;
use std::collections::HashMap;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use wayland_client::backend::Backend;
use wayland_client::protocol::{
    wl_compositor::WlCompositor, wl_shm::WlShm, wl_subcompositor::WlSubcompositor,
};
use wayland_client::{Connection, EventQueue, QueueHandle};
use wayland_protocols::xdg::shell::client::{xdg_surface::XdgSurface, xdg_wm_base::XdgWmBase};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::{
    Layer, ZwlrLayerShellV1,
};

pub struct Fake {
    _server: UnixStream,
    _connection: Connection,
    _queue: EventQueue<WaylandState>,
    pub qh: QueueHandle<WaylandState>,
    compositor: WlCompositor,
    subcompositor: WlSubcompositor,
    shm: WlShm,
    xdg_wm_base: XdgWmBase,
    layer_shell: ZwlrLayerShellV1,
}

impl Fake {
    pub fn new() -> Self {
        let (client, server) = UnixStream::pair().expect("socket pair");
        let connection = Connection::from_backend(Backend::connect(client).expect("backend"));
        let queue = connection.new_event_queue::<WaylandState>();
        let qh = queue.handle();
        let registry = connection.display().get_registry(&qh, ());
        // Versions high enough for set_buffer_scale/damage_buffer/reposition.
        let compositor = registry.bind(1, 4, &qh, ());
        let subcompositor = registry.bind(2, 1, &qh, ());
        let shm = registry.bind(3, 1, &qh, ());
        let xdg_wm_base = registry.bind(4, 3, &qh, ());
        let layer_shell = registry.bind(5, 1, &qh, ());
        Self {
            _server: server,
            _connection: connection,
            _queue: queue,
            qh,
            compositor,
            subcompositor,
            shm,
            xdg_wm_base,
            layer_shell,
        }
    }

    /// A `WaylandState` with every global bound and `popup.embed = embed`.
    pub fn state(&self, embed: PopupEmbed) -> WaylandState {
        let popup = PopupConfig::default();
        let config = Config::new(
            RootConfig::default(),
            ModulesConfig::default(),
            RenderingMode::default(),
            MetricsConfig::default(),
            TooltipConfig::default(),
            PopupConfig::new(popup.behavior(), popup.offset(), embed),
        );
        let tmp = || PathBuf::from("/tmp");
        let app_env = Arc::new(AppEnvironment::new(
            HomeDir::new(tmp()),
            XdgCacheHome::new(tmp()),
            XdgRuntimeDir::new(tmp()),
            RustLog::new(String::new()),
            None,
        ));
        let (command_tx, _command_rx) = tokio::sync::mpsc::channel(1);
        WaylandState {
            hub: Arc::new(SignalHub::new(config)),
            compositor: Some(self.compositor.clone()),
            subcompositor: Some(self.subcompositor.clone()),
            layer_shell: Some(self.layer_shell.clone()),
            xdg_wm_base: Some(self.xdg_wm_base.clone()),
            shm: Some(self.shm.clone()),
            outputs: Vec::new(),
            bars: Vec::new(),
            seat: None,
            pointer: None,
            command_tx,
            surface_to_id: HashMap::new(),
            pointer_surface: None,
            pointer_pos: (0.0, 0.0),
            floating_surfaces: HashMap::new(),
            pending_children: HashMap::new(),
            last_button_serial: None,
            app_env,
        }
    }

    pub fn shm_buffer(&self, size: Size) -> ShmBuffer {
        ShmBuffer::new(
            &self.shm,
            size.width(),
            size.height(),
            &self.qh,
            Path::new("/tmp"),
        )
        .expect("shm buffer")
    }

    pub fn xdg_surface(&self) -> XdgSurface {
        let surface = self.compositor.create_surface(&self.qh, ());
        self.xdg_wm_base.get_xdg_surface(&surface, &self.qh, ())
    }

    /// An already-shown floating surface of `size` whose owner frame is `base`.
    pub fn floating(&self, size: Size, base: RenderBuffer) -> FloatingSurface {
        let surface = self.compositor.create_surface(&self.qh, ());
        let xdg_surface = self.xdg_wm_base.get_xdg_surface(&surface, &self.qh, ());
        let positioner = self.xdg_wm_base.create_positioner(&self.qh, ());
        let xdg_popup = xdg_surface.get_popup(None, &positioner, &self.qh, ());
        FloatingSurface {
            surface,
            xdg_surface,
            xdg_popup,
            shm_buffer: self.shm_buffer(size),
            size,
            reposition_token: 0,
            module_surfaces: HashMap::new(),
            parent: None,
            base,
            layers: CompositeLayers::default(),
        }
    }

    /// A bar on output `name` with buffer scale `scale`.
    pub fn bar(&self, name: &str, scale: i32) -> WaylandBar {
        let surface = self.compositor.create_surface(&self.qh, ());
        let layer_surface = self.layer_shell.get_layer_surface(
            &surface,
            None,
            Layer::Top,
            "cranky".to_string(),
            &self.qh,
            (),
        );
        WaylandBar {
            output_name: name.to_string(),
            surface,
            layer_surface,
            shm_buffer: self.shm_buffer(Size::new(1, 1)),
            width: 1,
            height: 1,
            config_height: 1,
            config_margin: MarginConfig::default(),
            scale,
            module_surfaces: HashMap::new(),
            configured: true,
        }
    }
}

pub fn popup_kind() -> FloatingKind {
    FloatingKind::Popup(PopupTarget::new(ModuleId::new(1), MonitorId::new("DP-1")))
}

/// A `w`×`h` buffer filled with one premultiplied ARGB pixel.
pub fn solid(w: u32, h: u32, px: [u8; 4]) -> RenderBuffer {
    let n = usize::try_from(w.saturating_mul(h)).unwrap_or(0);
    RenderBuffer::new(px.repeat(n), Size::new(w, h))
}

/// Bytes of the slot `commit_surface` just attached (it swaps afterwards).
pub fn committed_bytes(shm: &mut ShmBuffer) -> Vec<u8> {
    shm.swap_buffers();
    let bytes = shm.mmap_mut().to_vec();
    shm.swap_buffers();
    bytes
}
