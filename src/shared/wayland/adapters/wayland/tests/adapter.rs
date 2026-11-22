use crate::shared::env::domain::AppEnvironment;
use crate::shared::primitives::geometry::{Position, Size};
use crate::shared::primitives::render::{ModuleFrame, PlacedBuffer, RenderBuffer};
use crate::shared::primitives::{ModuleId, MonitorId};
use crate::shared::wayland::adapters::wayland::tests::fixture::environment_with_runtime_dir;
use std::collections::HashMap;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use wayland_client::Connection;

use crate::shared::config::domain::Config;
use crate::shared::events::signals::SignalHub;
use crate::shared::wayland::adapters::wayland::adapter::{WaylandAdapter, readiness_socket};
use crate::shared::wayland::adapters::wayland::command::SurfaceCommand;
use crate::shared::wayland::adapters::wayland::state::WaylandState;
use crate::shared::wayland::adapters::wayland::surface_handler::handle_cmd;
use crate::shared::wayland::ports::DisplayServerPort;

/// An environment whose home, cache and runtime directories are `/tmp`.
fn tmp_environment() -> Arc<AppEnvironment> {
    environment_with_runtime_dir(PathBuf::from("/tmp"))
}

#[test]
fn test_wayland_state_initialization() {
    crate::test_utils::block_on(async {
        let (command_tx, _) = tokio::sync::mpsc::channel(10);
        let hub = Arc::new(SignalHub::new(Config::default()));
        let state = WaylandState::new(hub, command_tx, tmp_environment());
        assert!(state.bars.is_empty());
        assert!(state.outputs.is_empty());

        let _ = &state.hub;
        let _ = &state.subcompositor;
        let _ = &state.seat;
        let _ = &state.pointer;
        let _ = &state.command_tx;
    });
}

/// An adapter over a state with no global bound, on a connection nobody
/// serves.
fn unserved_adapter(hub: &Arc<SignalHub>) -> WaylandAdapter {
    let (client_stream, _) = UnixStream::pair().unwrap();
    let backend = wayland_client::backend::Backend::connect(client_stream).unwrap();
    let connection = Connection::from_backend(backend);
    let (command_tx, _) = tokio::sync::mpsc::channel(10);
    let (_, notify_rx) = tokio::sync::mpsc::channel(1);
    WaylandAdapter {
        event_queue: connection.new_event_queue::<WaylandState>(),
        socket: readiness_socket(&connection).unwrap(),
        connection,
        state: WaylandState::new(hub.clone(), command_tx, tmp_environment()),
        pending_surfaces: Arc::new(Mutex::new(HashMap::new())),
        notify_rx,
        config_rx: hub.config_rx(),
    }
}

#[test]
fn test_wayland_adapter_methods() {
    crate::test_utils::block_on(async {
        let hub = Arc::new(SignalHub::new(Config::default()));
        let mut adapter = unserved_adapter(&hub);

        assert!(adapter.flush().is_ok());

        let _ = adapter.dispatch_pending();

        let cmd = SurfaceCommand::new(
            ModuleFrame::new(
                ModuleId::new(1),
                MonitorId::new("test"),
                PlacedBuffer::new(
                    Position::new(0, 0),
                    RenderBuffer::new(vec![0; 4], Size::new(1, 1)),
                ),
            ),
            None,
        );
        let queue_handle = adapter.event_queue.handle();
        let _ = handle_cmd(&mut adapter.state, &queue_handle, &cmd);

        let (tx, _rx) = tokio::sync::mpsc::channel(10);
        let _ = WaylandAdapter::new(hub, tx, tmp_environment());
    });
}

#[test]
fn test_wayland_output_scale_broadcast() {
    let hub = Arc::new(SignalHub::new(Config::default()));
    let scales_rx = hub.monitor_scales_rx();

    let mut scales = scales_rx.borrow().clone();
    scales.insert(
        crate::shared::primitives::MonitorId::new("eDP-1"),
        crate::shared::primitives::geometry::Scale::new(2.0),
    );
    hub.monitor_scales_tx().send(scales).unwrap();

    assert_eq!(
        scales_rx
            .borrow()
            .get(&crate::shared::primitives::MonitorId::new("eDP-1")),
        Some(&crate::shared::primitives::geometry::Scale::new(2.0))
    );
}
