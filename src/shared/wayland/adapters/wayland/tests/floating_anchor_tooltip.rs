use crate::shared::wayland::adapters::wayland::dispatch_pointer_button::PointerGrab;
use crate::features::layout_engine::domain::{FloatingKind, PopupTarget};
use crate::shared::config::domain::Config;
use crate::shared::env::domain::{AppEnvironment, HomeDir, RustLog, XdgCacheHome, XdgRuntimeDir};
use crate::shared::events::signals::SignalHub;
use crate::shared::config::domain::PopupEmbed;
use crate::shared::primitives::geometry::{Position, Rect, Size};
use crate::shared::wayland::adapters::wayland::dispatch_pointer::handle_pointer_enter;
use crate::shared::wayland::adapters::wayland::tests::fixture::pointer::{CHILD, popup};
use crate::shared::wayland::adapters::wayland::tests::fixture::{Fake, popup_kind, solid};
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
        pointer_target: None,
        pointer_grab: PointerGrab::default(),
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

/// A 50×20 popup owned by module 1 on DP-1 with module 9 composited as a
/// 4×4 layer at (30, 10).
fn popup_with_layer_at_30_10(fake: &Fake) -> WaylandState {
    let mut state = fake.state(PopupEmbed::Composite);
    state.bars.push(fake.bar("DP-1", 1));
    let mut floating = fake.floating(Size::new(50, 20), solid(50, 20, [0; 4]));
    let (id, monitor) = (ModuleId::new(CHILD), MonitorId::new("DP-1"));
    let (origin, buffer) = (Position::new(30, 10), solid(4, 4, [1; 4]));
    floating.layers.upsert(id, monitor, origin, buffer);
    state.floating_surfaces.insert(popup_kind(), floating);
    state
}

#[test]
fn nested_popup_anchor_uses_composite_child_origin() {
    let fake = Fake::new();
    let state = popup_with_layer_at_30_10(&fake);
    let target = PopupTarget::new(ModuleId::new(CHILD), MonitorId::new("DP-1"));
    let (kind, parent) = (FloatingKind::Popup(target), popup_kind());
    let bounds = |rect| {
        resolve_anchor(&state, &kind, None, rect, Some(&parent))
            .map(|a| (a.anchor_x, a.anchor_y, a.anchor_w, a.anchor_h))
    };
    let rect = Rect::new(Position::new(2, 3), Size::new(5, 6));
    assert_eq!(bounds(Some(rect)), Some((32, 13, 5, 6)));
    assert_eq!(bounds(None), Some((30, 10, 4, 4)));
}

#[test]
fn tooltip_anchor_resolves_for_composite_child() {
    let fake = Fake::new();
    let (mut state, surface) = popup(&fake, 1);
    handle_pointer_enter(&mut state, &surface, 12.0, 4.0);
    let anchor = resolve_anchor(&state, &FloatingKind::Tooltip, None, None, None)
        .expect("tooltip anchor resolves for a composited child");
    let popup_xdg = state
        .floating_surfaces
        .get(&popup_kind())
        .map(|f| f.xdg_surface.clone());
    assert!(
        matches!(&anchor.parent_role, FloatingParentRole::Popup(x) if Some(x) == popup_xdg.as_ref())
    );
    assert_eq!(anchor.target_monitor_id, MonitorId::new("DP-1"));
    // Already popup-local: no layer origin added on top.
    assert_eq!(anchor.anchor_x, 12);
}

#[test]
fn tooltip_anchor_ignores_popup_owner_own_content() {
    let fake = Fake::new();
    let (mut state, surface) = popup(&fake, 1);
    handle_pointer_enter(&mut state, &surface, 1.0, 1.0);
    assert!(resolve_anchor(&state, &FloatingKind::Tooltip, None, None, None).is_none());
}
