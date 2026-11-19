use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::shared::wayland::adapters::wayland::command::SurfaceCommand;
use crate::shared::wayland::adapters::wayland::surface_manager::WaylandSurfaceManager;
use crate::shared::wayland::ports::SurfaceManagerPort;

#[test]
fn test_wayland_surface_manager() {
    crate::test_utils::block_on(async {
        let pending_surfaces = Arc::new(Mutex::new(HashMap::new()));
        let (notify_tx, mut notify_rx) = tokio::sync::mpsc::channel(1);
        let manager = WaylandSurfaceManager {
            pending_surfaces: pending_surfaces.clone(),
            notify_tx,
        };

        let module_id = crate::shared::primitives::ModuleId::new(1);
        let monitor_id = crate::shared::primitives::MonitorId::new("DP-1");
        let buffer = crate::shared::primitives::render::RenderBuffer::new(
            vec![0; 400],
            crate::shared::primitives::geometry::Size::new(10, 10),
        );

        manager.submit_buffer(
            module_id,
            monitor_id.clone(),
            crate::shared::primitives::geometry::Position::new(0, 0),
            buffer,
        );

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
        let pending_surfaces = Arc::new(Mutex::new(HashMap::new()));
        let (notify_tx, mut notify_rx) = tokio::sync::mpsc::channel(1);
        let manager = WaylandSurfaceManager {
            pending_surfaces: pending_surfaces.clone(),
            notify_tx,
        };

        let module_id = crate::shared::primitives::ModuleId::new(1);
        let monitor_id = crate::shared::primitives::MonitorId::new("DP-1");
        let buffer1 = crate::shared::primitives::render::RenderBuffer::new(
            vec![0; 400],
            crate::shared::primitives::geometry::Size::new(10, 10),
        );
        let buffer2 = crate::shared::primitives::render::RenderBuffer::new(
            vec![0; 1600],
            crate::shared::primitives::geometry::Size::new(20, 20),
        );

        manager.submit_buffer(
            module_id,
            monitor_id.clone(),
            crate::shared::primitives::geometry::Position::new(0, 0),
            buffer1,
        );
        manager.submit_buffer(
            module_id,
            monitor_id.clone(),
            crate::shared::primitives::geometry::Position::new(0, 0),
            buffer2,
        );

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
        module_id,
        None,
        monitor_id.clone(),
        crate::shared::primitives::geometry::Position::new(0, 0),
        buffer,
    );

    assert_eq!(cmd.module_id(), module_id);
    assert_eq!(cmd.monitor_id(), &monitor_id);
    assert_eq!(cmd.buffer().size().height(), 20);
}
