use crate::shared::primitives::render::{ModuleFrame, PlacedBuffer};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::shared::primitives::{ModuleId, MonitorId};
use crate::shared::wayland::adapters::wayland::command::SurfaceCommand;
use crate::shared::wayland::adapters::wayland::surface_manager::WaylandSurfaceManager;
use crate::shared::wayland::ports::SurfaceManagerPort;
use tokio::sync::mpsc::Receiver;

/// The commands waiting for the adapter, per module and monitor.
type PendingSurfaces = Arc<Mutex<HashMap<(ModuleId, MonitorId), SurfaceCommand>>>;

/// A manager over an empty pending map, with that map and the receiver of
/// its wake-ups.
fn manager() -> (WaylandSurfaceManager, PendingSurfaces, Receiver<()>) {
    let pending_surfaces = Arc::new(Mutex::new(HashMap::new()));
    let (notify_tx, notify_rx) = tokio::sync::mpsc::channel(1);
    let manager = WaylandSurfaceManager {
        pending_surfaces: pending_surfaces.clone(),
        notify_tx,
    };
    (manager, pending_surfaces, notify_rx)
}

/// Module 1's frame on DP-1: a `side`×`side` buffer at the origin.
fn frame(side: u32) -> ModuleFrame {
    let len = usize::try_from(side.saturating_mul(side).saturating_mul(4)).unwrap();
    ModuleFrame::new(
        crate::shared::primitives::ModuleId::new(1),
        crate::shared::primitives::MonitorId::new("DP-1"),
        PlacedBuffer::new(
            crate::shared::primitives::geometry::Position::new(0, 0),
            crate::shared::primitives::render::RenderBuffer::new(
                vec![0; len],
                crate::shared::primitives::geometry::Size::new(side, side),
            ),
        ),
    )
}

#[test]
fn test_wayland_surface_manager() {
    crate::test_utils::block_on(async {
        let (manager, pending_surfaces, mut notify_rx) = manager();
        let module_id = crate::shared::primitives::ModuleId::new(1);
        let monitor_id = crate::shared::primitives::MonitorId::new("DP-1");

        manager.submit_buffer(frame(10));

        notify_rx
            .recv()
            .await
            .expect("Failed to receive notification");
        let cmd = pending_surfaces
            .lock()
            .unwrap()
            .remove(&(module_id, monitor_id))
            .expect("Failed to find command");
        assert_eq!(cmd.module_id(), module_id);
        assert_eq!(cmd.monitor_id().as_str(), "DP-1");
        assert_eq!(cmd.buffer().size().width(), 10);
    });
}

#[test]
fn test_wayland_surface_manager_coalesces_latest_frame() {
    crate::test_utils::block_on(async {
        let (manager, pending_surfaces, mut notify_rx) = manager();
        let module_id = crate::shared::primitives::ModuleId::new(1);
        let monitor_id = crate::shared::primitives::MonitorId::new("DP-1");

        manager.submit_buffer(frame(10));
        manager.submit_buffer(frame(20));

        notify_rx
            .recv()
            .await
            .expect("Failed to receive notification");
        let mut map = pending_surfaces.lock().unwrap();
        assert_eq!(map.len(), 1);
        let cmd = map
            .remove(&(module_id, monitor_id))
            .expect("Failed to find command");
        drop(map);
        assert_eq!(cmd.buffer().size().width(), 20);
    });
}

#[test]
fn test_surface_command_struct() {
    let module_id = crate::shared::primitives::ModuleId::new(2);
    let monitor_id = crate::shared::primitives::MonitorId::new("HDMI-1");
    let buffer = crate::shared::primitives::render::RenderBuffer::new(
        vec![0; 1600],
        crate::shared::primitives::geometry::Size::new(20, 20),
    );
    let cmd = SurfaceCommand::new(
        ModuleFrame::new(
            module_id,
            monitor_id.clone(),
            PlacedBuffer::new(
                crate::shared::primitives::geometry::Position::new(0, 0),
                buffer,
            ),
        ),
        None,
    );

    assert_eq!(cmd.module_id(), module_id);
    assert_eq!(cmd.monitor_id(), &monitor_id);
    assert_eq!(cmd.buffer().size().height(), 20);
}
