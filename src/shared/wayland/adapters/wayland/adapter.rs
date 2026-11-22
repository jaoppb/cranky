use super::command::SurfaceCommand;
use super::state::WaylandState;
use super::surface_manager::WaylandSurfaceManager;
use crate::features::layout_engine::domain::DisplayCommand;
use crate::shared::config::domain::Config;
use crate::shared::env::domain::AppEnvironment;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::{ModuleId, MonitorId};
use crate::shared::wayland::ports::DisplayServerError;
use std::collections::HashMap;
use std::os::unix::io::AsFd;
use std::os::unix::net::UnixStream as StdUnixStream;
use std::sync::{Arc, Mutex};
use tokio::net::UnixStream;
use wayland_client::{Connection, EventQueue};

/// Owns the Wayland connection and event queue, and serves as the app's display server.
///
/// It attaches the buffers its `WaylandSurfaceManager` queues whenever that handle wakes it through
/// `notify_rx`.
pub struct WaylandAdapter {
    pub(crate) connection: Connection,
    pub(crate) event_queue: EventQueue<WaylandState>,
    pub(crate) state: WaylandState,
    /// A duplicate of the connection's socket, registered with tokio only to
    /// learn when the connection has events to read.
    pub(crate) socket: UnixStream,
    pub(crate) pending_surfaces: Arc<Mutex<HashMap<(ModuleId, MonitorId), SurfaceCommand>>>,
    pub(crate) notify_rx: tokio::sync::mpsc::Receiver<()>,
    pub(crate) config_rx: tokio::sync::watch::Receiver<Config>,
}

impl std::fmt::Debug for WaylandAdapter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WaylandAdapter")
            .finish_non_exhaustive()
    }
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
        let connection = Connection::connect_to_env().map_err(|error| connection_failed(&error))?;
        let event_queue = connection.new_event_queue();
        let queue_handle = event_queue.handle();

        connection.display().get_registry(&queue_handle, ());

        let socket = readiness_socket(&connection).map_err(|error| connection_failed(&error))?;

        let config_rx = hub.config_rx();
        let state = WaylandState::new(hub, command_tx, app_env);

        let pending_surfaces = Arc::new(Mutex::new(HashMap::new()));
        let (notify_tx, notify_rx) = tokio::sync::mpsc::channel(1);

        let adapter = Self {
            connection,
            event_queue,
            state,
            socket,
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

/// The connection failure `error` stands for.
pub(super) fn connection_failed(error: &impl ToString) -> DisplayServerError {
    DisplayServerError::ConnectionFailed {
        reason: error.to_string(),
    }
}

/// Duplicates the connection's socket into a tokio stream. The duplicate
/// shares the open socket, so its readiness is the connection's readiness;
/// nothing is ever read from or written to it. Making the duplicate
/// non-blocking makes the connection's socket non-blocking too, which changes
/// nothing: `wayland-backend` always reads and writes with `MSG_DONTWAIT`.
pub(super) fn readiness_socket(connection: &Connection) -> std::io::Result<UnixStream> {
    let socket = StdUnixStream::from(connection.as_fd().try_clone_to_owned()?);
    socket.set_nonblocking(true)?;
    UnixStream::from_std(socket)
}
