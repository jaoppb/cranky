use crate::features::workspaces::adapters::hyprland_adapter::HyprlandAdapter;
use crate::features::workspaces::adapters::hyprland_provider::MockHyprlandProvider;
use crate::features::workspaces::ports::WindowManagerPort;
use crate::shared::events::signals::SignalHub;
use std::sync::Arc;

#[tokio::test]
async fn test_hyprland_adapter_get_state() {
    let mut mock_provider = MockHyprlandProvider::new();
    mock_provider
        .expect_query_workspaces()
        .times(1)
        .returning(|| Ok("[]".to_string()));
    mock_provider
        .expect_query_monitors()
        .times(1)
        .returning(|| Ok("[]".to_string()));

    let adapter = HyprlandAdapter::with_provider(Arc::new(mock_provider));

    let res = adapter.get_state().unwrap();
    assert_eq!(res.0.len(), 0);
    assert_eq!(res.1.len(), 0);
    assert_eq!(res.2, None);
}

#[tokio::test]
async fn test_hyprland_adapter_get_state_valid() {
    let mut mock_provider = MockHyprlandProvider::new();
    mock_provider
        .expect_query_workspaces()
        .times(1)
        .returning(|| Ok(r#"[{"id": 1, "name": "1", "monitor": "DP-1"}]"#.to_string()));
    mock_provider
        .expect_query_monitors()
        .times(1)
        .returning(|| Ok(r#"[{"name": "DP-1", "activeWorkspace": {"id": 1}, "specialWorkspace": {"id": 0}, "focused": true}]"#.to_string()));

    let adapter = HyprlandAdapter::with_provider(Arc::new(mock_provider));

    let res = adapter.get_state().unwrap();
    assert_eq!(res.0.len(), 1); // workspaces
    assert_eq!(res.1.len(), 1); // monitors
    assert_eq!(
        res.2,
        Some(crate::features::workspaces::domain::MonitorName::new(
            "DP-1"
        ))
    ); // focused
}

#[tokio::test]
async fn test_hyprland_adapter_run() {
    use std::io::Write;
    use std::os::unix::net::UnixStream;

    let (mut sender, receiver) = UnixStream::pair().unwrap();

    let mut mock_provider = MockHyprlandProvider::new();
    mock_provider
        .expect_query_workspaces()
        .returning(|| Ok("[]".to_string()));
    mock_provider
        .expect_query_monitors()
        .returning(|| Ok("[]".to_string()));

    mock_provider
        .expect_listen_events()
        .times(1)
        .returning(move || {
            let stream = receiver.try_clone().unwrap();
            Ok(stream)
        });

    let adapter = HyprlandAdapter::with_provider(Arc::new(mock_provider));

    let config = crate::shared::config::domain::Config::default();
    let hub = Arc::new(SignalHub::new(config));

    let run_handle = tokio::task::spawn(adapter.run(hub.clone()));

    sender.write_all(b"workspacev2>>1,test_ws\n").unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    drop(sender);

    let _ = run_handle.await;
}
