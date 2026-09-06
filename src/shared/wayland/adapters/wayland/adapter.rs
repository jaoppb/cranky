use super::command::SurfaceCommand;
use super::state::WaylandState;
use super::surface_manager::WaylandSurfaceManager;
use super::types::WaylandFd;
use crate::features::layout_engine::domain::DisplayCommand;
use crate::shared::config::domain::Config;
use crate::shared::env::domain::AppEnvironment;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::{ModuleId, MonitorId};
use crate::shared::wayland::ports::DisplayServerError;
use cosmic_text::{FontSystem, SwashCache};
use std::collections::HashMap;
use std::os::unix::io::{AsFd, AsRawFd};
use std::sync::{Arc, Mutex};
use tokio::io::unix::AsyncFd;
use wayland_client::{Connection, EventQueue};

pub struct WaylandAdapter {
    pub(crate) connection: Connection,
    pub(crate) event_queue: EventQueue<WaylandState>,
    pub(crate) state: WaylandState,
    pub(crate) async_fd: AsyncFd<WaylandFd>,
    pub(crate) pending_surfaces: Arc<Mutex<HashMap<(ModuleId, MonitorId), SurfaceCommand>>>,
    pub(crate) notify_rx: tokio::sync::mpsc::Receiver<()>,
    pub(crate) config_rx: tokio::sync::watch::Receiver<Config>,
}

impl WaylandAdapter {
    /// # Errors
    ///
    /// Returns `DisplayServerError::ConnectionFailed` if connecting to Wayland environment fails.
    pub fn new(
        hub: Arc<SignalHub>,
        command_tx: tokio::sync::mpsc::Sender<DisplayCommand>,
        app_env: Arc<AppEnvironment>,
    ) -> Result<(Self, WaylandSurfaceManager), DisplayServerError> {
        let connection =
            Connection::connect_to_env().map_err(|e| DisplayServerError::ConnectionFailed {
                reason: e.to_string(),
            })?;
        let event_queue = connection.new_event_queue();
        let qh = event_queue.handle();

        connection.display().get_registry(&qh, ());

        let raw_fd = connection.as_fd().as_raw_fd();
        let async_fd =
            AsyncFd::new(WaylandFd(raw_fd)).map_err(|e| DisplayServerError::ConnectionFailed {
                reason: e.to_string(),
            })?;

        let config_rx = hub.config_rx();
        let state = WaylandState {
            hub,
            compositor: None,
            shm: None,
            layer_shell: None,
            subcompositor: None,
            xdg_wm_base: None,
            outputs: Vec::new(),
            bars: Vec::new(),
            seat: None,
            pointer: None,
            command_tx,
            surface_to_id: HashMap::new(),
            pointer_surface: None,
            pointer_pos: (0.0, 0.0),
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
            floating_surfaces: HashMap::new(),
            app_env,
        };

        let pending_surfaces = Arc::new(Mutex::new(HashMap::new()));
        let (notify_tx, notify_rx) = tokio::sync::mpsc::channel(1);

        let adapter = Self {
            connection,
            event_queue,
            state,
            async_fd,
            pending_surfaces: pending_surfaces.clone(),
            notify_rx,
            config_rx,
        };

        let manager = WaylandSurfaceManager {
            pending_surfaces,
            notify_tx,
        };

        Ok((adapter, manager))
    }
}
