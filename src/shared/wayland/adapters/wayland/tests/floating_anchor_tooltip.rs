use crate::features::layout_engine::domain::{FloatingKind, PopupTarget};
use crate::shared::config::domain::Config;
use crate::shared::env::domain::{AppEnvironment, HomeDir, RustLog, XdgCacheHome, XdgRuntimeDir};
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::{ModuleId, MonitorId};
use crate::shared::wayland::adapters::shm::ShmBuffer;
use crate::shared::wayland::adapters::wayland::floating_anchor::{
    resolve_anchor, FloatingParentRole,
};
use crate::shared::wayland::adapters::wayland::state::WaylandState;
use crate::shared::wayland::adapters::wayland::types::{FloatingSurface, ModuleSurface};
use std::collections::HashMap;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::Arc;
use wayland_client::backend::Backend;
use wayland_client::protocol::{wl_compositor::WlCompositor, wl_shm::WlShm, wl_subcompositor::WlSubcompositor};
use wayland_client::Connection;
use wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase;

/// Regression test (Gap A): the hovered element can belong to a module
/// that isn't embedded in the bar at all, but in a module embedded inside
/// an already-open popup (`ui.module()` nested inside floating content).
/// `resolve_anchor` must find it by also scanning
/// `WaylandState::floating_surfaces` for a `module_surfaces` match, the
/// same way the `Popup`/`Panel` arms already scan a bar's — not just
/// return `None` because no bar owns the hovered surface.
#[test]
fn test_resolve_anchor_tooltip_finds_module_embedded_in_open_popup() {
    let (client_stream, _server_stream) = UnixStream::pair().expect("socket pair");
    let backend = Backend::connect(client_stream).expect("connect backend");
    let connection = Connection::from_backend(backend);
    let event_queue = connection.new_event_queue::<WaylandState>();
    let qh = event_queue.handle();

    let display = connection.display();
    let registry = display.get_registry(&qh, ());
    let compositor: WlCompositor = registry.bind(1, 1, &qh, ());
    let subcompositor: WlSubcompositor = registry.bind(2, 1, &qh, ());
    let shm: WlShm = registry.bind(3, 1, &qh, ());
    let xdg_wm_base: XdgWmBase = registry.bind(4, 1, &qh, ());

    // The already-open popup's own surface/xdg_surface — what the embedded
    // module's tooltip must nest inside.
    let popup_surface = compositor.create_surface(&qh, ());
    let popup_xdg_surface = xdg_wm_base.get_xdg_surface(&popup_surface, &qh, ());
    let positioner = xdg_wm_base.create_positioner(&qh, ());
    let popup_xdg_popup = popup_xdg_surface.get_popup(None, &positioner, &qh, ());
    let popup_shm_buffer =
        ShmBuffer::new(&shm, 2, 2, &qh, Path::new("/tmp")).expect("popup shm buffer");

    // The module embedded inside that popup's own content (decision 7, gap
    // 4): its own subsurface, parented to the popup's surface rather than
    // to a bar's.
    let module_surface = compositor.create_surface(&qh, ());
    let module_subsurface = subcompositor.get_subsurface(&module_surface, &popup_surface, &qh, ());
    let module_shm_buffer =
        ShmBuffer::new(&shm, 2, 2, &qh, Path::new("/tmp")).expect("module shm buffer");

    let mut module_surfaces = HashMap::new();
    module_surfaces.insert(
        ModuleId::new(7),
        ModuleSurface {
            surface: module_surface.clone(),
            subsurface: module_subsurface,
            shm_buffer: module_shm_buffer,
            size: Size::new(2, 2),
            x: 5,
            y: 3,
        },
    );

    let mut floating_surfaces = HashMap::new();
    floating_surfaces.insert(
        FloatingKind::Popup(PopupTarget::new(ModuleId::new(1), MonitorId::new("DP-1"))),
        FloatingSurface {
            surface: popup_surface,
            xdg_surface: popup_xdg_surface,
            xdg_popup: popup_xdg_popup,
            shm_buffer: popup_shm_buffer,
            size: Size::new(100, 100),
            reposition_token: 0,
            module_surfaces,
            parent: None,
            base: crate::shared::primitives::render::RenderBuffer::new(Vec::new(), Size::new(0, 0)),
            layers: crate::shared::wayland::composite::CompositeLayers::default(),
            configure: crate::shared::wayland::adapters::wayland::types::ConfigureGate::default(),
        },
    );

    let (command_tx, _command_rx) = tokio::sync::mpsc::channel(1);
    let app_env = Arc::new(AppEnvironment::new(
        HomeDir::new(std::path::PathBuf::from("/tmp")),
        XdgCacheHome::new(std::path::PathBuf::from("/tmp")),
        XdgRuntimeDir::new(std::path::PathBuf::from("/tmp")),
        RustLog::new(String::new()),
        None,
    ));
    let state = WaylandState {
        hub: Arc::new(SignalHub::new(Config::default())),
        compositor: None,
        subcompositor: None,
        layer_shell: None,
        xdg_wm_base: None,
        shm: None,
        outputs: Vec::new(),
        bars: Vec::new(),
        seat: None,
        pointer: None,
        command_tx,
        surface_to_id: HashMap::new(),
        // The pointer is over the embedded module's own subsurface, not
        // any bar surface.
        pointer_surface: Some(module_surface),
        pointer_pos: (12.0, 4.0),
        floating_surfaces,
        pending_children: HashMap::new(),
        last_button_serial: None,
        app_env,
    };

    let anchor = resolve_anchor(&state, &FloatingKind::Tooltip, None, None, None)
        .expect("tooltip anchor must resolve for a module embedded in an open popup");

    assert!(
        matches!(anchor.parent_role, FloatingParentRole::Popup(_)),
        "tooltip must nest inside the popup's xdg_surface, not fall back to a bar's layer surface"
    );
    assert_eq!(anchor.target_monitor_id, MonitorId::new("DP-1"));
    // pointer_pos.0 (12) + the embedded module's own subsurface offset
    // (5), exactly the transform the bar-owned case already applies.
    assert_eq!(anchor.anchor_x, 17);

    drop(event_queue);
}
