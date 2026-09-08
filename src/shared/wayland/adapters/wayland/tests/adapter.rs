use std::collections::HashMap;
use std::os::unix::io::{AsFd, AsRawFd};
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};

use cosmic_text::{FontSystem, SwashCache};
use wayland_client::Connection;

use crate::shared::config::domain::Config;
use crate::shared::events::signals::SignalHub;
use crate::shared::wayland::adapters::wayland::adapter::WaylandAdapter;
use crate::shared::wayland::adapters::wayland::command::SurfaceCommand;
use crate::shared::wayland::adapters::wayland::state::WaylandState;
use crate::shared::wayland::adapters::wayland::surface_handler::handle_cmd;
use crate::shared::wayland::adapters::wayland::types::WaylandFd;
use crate::shared::wayland::ports::DisplayServerPort;

#[tokio::test]
async fn test_wayland_state_initialization() {
    let (command_tx, _) = tokio::sync::mpsc::channel(10);
    let app_env = std::sync::Arc::new(crate::shared::env::domain::AppEnvironment::new(
        crate::shared::env::domain::HomeDir::new(std::path::PathBuf::from("/tmp")),
        crate::shared::env::domain::XdgCacheHome::new(std::path::PathBuf::from("/tmp")),
        crate::shared::env::domain::XdgRuntimeDir::new(std::path::PathBuf::from("/tmp")),
        crate::shared::env::domain::RustLog::new(String::new()),
        None,
    ));
    let state = WaylandState {
        hub: Arc::new(SignalHub::new(
            crate::shared::config::domain::Config::default(),
        )),
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
        last_button_serial: None,
        app_env,
    };
    assert!(state.bars.is_empty());
    assert!(state.outputs.is_empty());

    let _ = &state.hub;
    let _ = &state.subcompositor;
    let _ = &state.seat;
    let _ = &state.pointer;
    let _ = &state.command_tx;
}

#[tokio::test]
async fn test_wayland_adapter_methods() {
    let (client_stream, _) = UnixStream::pair().unwrap();
    let backend = wayland_client::backend::Backend::connect(client_stream).unwrap();
    let connection = Connection::from_backend(backend);
    let event_queue = connection.new_event_queue::<WaylandState>();
    let async_fd =
        tokio::io::unix::AsyncFd::new(WaylandFd(connection.as_fd().as_raw_fd())).unwrap();

    let (command_tx, _) = tokio::sync::mpsc::channel(10);
    let config = crate::shared::config::domain::Config::default();
    let hub = Arc::new(SignalHub::new(config));
    let config_rx = hub.config_rx();

    let app_env = std::sync::Arc::new(crate::shared::env::domain::AppEnvironment::new(
        crate::shared::env::domain::HomeDir::new(std::path::PathBuf::from("/tmp")),
        crate::shared::env::domain::XdgCacheHome::new(std::path::PathBuf::from("/tmp")),
        crate::shared::env::domain::XdgRuntimeDir::new(std::path::PathBuf::from("/tmp")),
        crate::shared::env::domain::RustLog::new(String::new()),
        None,
    ));
    let state = WaylandState {
        hub: hub.clone(),
        compositor: None,
        shm: None,
        layer_shell: None,
        xdg_wm_base: None,
        subcompositor: None,
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
        last_button_serial: None,
        app_env,
    };

    let pending_surfaces = Arc::new(Mutex::new(HashMap::new()));
    let (_, notify_rx) = tokio::sync::mpsc::channel(1);

    let mut adapter = WaylandAdapter {
        connection,
        event_queue,
        state,
        async_fd,
        pending_surfaces,
        notify_rx,
        config_rx,
    };

    assert!(adapter.flush().is_ok());

    let _ = adapter.dispatch_pending();

    let cmd = SurfaceCommand::new(
        crate::shared::primitives::ModuleId::new(1),
        None,
        crate::shared::primitives::MonitorId::new("test"),
        crate::shared::primitives::geometry::Position::new(0, 0),
        crate::shared::primitives::render::RenderBuffer::new(
            vec![0; 4],
            crate::shared::primitives::geometry::Size::new(1, 1),
        ),
    );
    let qh = adapter.event_queue.handle();
    let _ = handle_cmd(&mut adapter.state, &qh, &cmd);

    let (tx, _rx) = tokio::sync::mpsc::channel(10);
    let app_env = std::sync::Arc::new(crate::shared::env::domain::AppEnvironment::new(
        crate::shared::env::domain::HomeDir::new(std::path::PathBuf::from("/tmp")),
        crate::shared::env::domain::XdgCacheHome::new(std::path::PathBuf::from("/tmp")),
        crate::shared::env::domain::XdgRuntimeDir::new(std::path::PathBuf::from("/tmp")),
        crate::shared::env::domain::RustLog::new(String::new()),
        None,
    ));
    let _ = WaylandAdapter::new(hub, tx, app_env);
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
