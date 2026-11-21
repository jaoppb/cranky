use crate::features::workspaces::adapters::hyprland_adapter::HyprlandAdapter;
use crate::features::workspaces::adapters::hyprland_provider::{HyprError, HyprlandProvider};
use crate::features::workspaces::ports::WindowManagerPort;
use crate::shared::events::signals::SignalHub;
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// A Hyprland that answers both queries with fixed JSON, hands out `events`
/// as its event socket once, and counts each kind of request.
struct FakeHyprland {
    workspaces: &'static str,
    monitors: &'static str,
    events: Mutex<Option<UnixStream>>,
    workspace_queries: AtomicUsize,
    monitor_queries: AtomicUsize,
    event_listens: AtomicUsize,
}

impl FakeHyprland {
    const fn new(workspaces: &'static str, monitors: &'static str) -> Self {
        Self {
            workspaces,
            monitors,
            events: Mutex::new(None),
            workspace_queries: AtomicUsize::new(0),
            monitor_queries: AtomicUsize::new(0),
            event_listens: AtomicUsize::new(0),
        }
    }

    /// The event socket, the first time it is asked for.
    ///
    /// The adapter reconnects forever on a blocking thread that nothing can
    /// cancel, and the test runtime waits for that thread when it shuts down.
    /// Panicking on the second request is the only way a test ends it.
    fn take_events(&self) -> UnixStream {
        self.events
            .lock()
            .unwrap()
            .take()
            .expect("the adapter asked for the event socket again")
    }

    /// How often workspaces, monitors and the event socket were requested.
    fn requests(&self) -> (usize, usize, usize) {
        (
            self.workspace_queries.load(Ordering::SeqCst),
            self.monitor_queries.load(Ordering::SeqCst),
            self.event_listens.load(Ordering::SeqCst),
        )
    }
}

impl HyprlandProvider for FakeHyprland {
    fn query_monitors(&self) -> Result<String, HyprError> {
        self.monitor_queries.fetch_add(1, Ordering::SeqCst);
        Ok(self.monitors.to_string())
    }

    fn query_workspaces(&self) -> Result<String, HyprError> {
        self.workspace_queries.fetch_add(1, Ordering::SeqCst);
        Ok(self.workspaces.to_string())
    }

    fn listen_events(&self) -> Result<UnixStream, HyprError> {
        self.event_listens.fetch_add(1, Ordering::SeqCst);
        Ok(self.take_events())
    }
}

#[test]
fn test_hyprland_adapter_get_state() {
    crate::test_utils::block_on(async {
        let provider = Arc::new(FakeHyprland::new("[]", "[]"));
        let adapter = HyprlandAdapter::with_provider(provider.clone());

        let res = adapter.get_state().unwrap();
        assert_eq!(provider.requests(), (1, 1, 0));
        assert_eq!(res.0.len(), 0);
        assert_eq!(res.1.len(), 0);
        assert_eq!(res.2, None);
    });
}

#[test]
fn test_hyprland_adapter_get_state_valid() {
    crate::test_utils::block_on(async {
        let provider = Arc::new(FakeHyprland::new(
            r#"[{"id": 1, "name": "1", "monitor": "DP-1"}]"#,
            r#"[{"name": "DP-1", "activeWorkspace": {"id": 1}, "specialWorkspace": {"id": 0}, "focused": true}]"#,
        ));
        let adapter = HyprlandAdapter::with_provider(provider.clone());

        let res = adapter.get_state().unwrap();
        assert_eq!(provider.requests(), (1, 1, 0));
        assert_eq!(res.0.len(), 1); // workspaces
        assert_eq!(res.1.len(), 1); // monitors
        assert_eq!(
            res.2,
            Some(crate::features::workspaces::domain::MonitorName::new(
                "DP-1"
            ))
        ); // focused
    });
}

/// Of several monitors, the one Hyprland marks focused is the focused one,
/// whatever its place in the reply.
#[test]
fn test_hyprland_adapter_get_state_picks_the_focused_monitor() {
    use crate::features::workspaces::domain::MonitorName;

    let provider = Arc::new(FakeHyprland::new(
        "[]",
        r#"[{"name": "DP-1", "activeWorkspace": {"id": 1}, "specialWorkspace": {"id": 0}, "focused": false},
            {"name": "HDMI-A-1", "activeWorkspace": {"id": 2}, "specialWorkspace": {"id": -98}, "focused": true},
            {"name": "eDP-1", "activeWorkspace": {"id": 3}, "specialWorkspace": {"id": 0}, "focused": false}]"#,
    ));
    let (_, monitors, focused) = HyprlandAdapter::with_provider(provider)
        .get_state()
        .unwrap();

    assert_eq!(monitors.len(), 3);
    assert_eq!(focused, Some(MonitorName::new("HDMI-A-1")));
    let hdmi = &monitors[&MonitorName::new("HDMI-A-1")];
    assert_eq!(
        hdmi.special_workspace_id(),
        Some(&crate::features::workspaces::domain::WorkspaceId::new(-98))
    );
}

#[test]
fn test_hyprland_adapter_run() {
    crate::test_utils::block_on(async {
        use std::io::Write;

        let (mut sender, receiver) = UnixStream::pair().unwrap();

        let provider = Arc::new(FakeHyprland::new("[]", "[]"));
        *provider.events.lock().unwrap() = Some(receiver);
        let adapter = HyprlandAdapter::with_provider(provider.clone());

        let config = crate::shared::config::domain::Config::default();
        let hub = Arc::new(SignalHub::new(config));

        let run_handle = tokio::task::spawn(adapter.run(hub.clone()));

        sender.write_all(b"workspacev2>>1,test_ws\n").unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        drop(sender);

        let _ = run_handle.await;
        assert_eq!(provider.requests().2, 1);
    });
}
