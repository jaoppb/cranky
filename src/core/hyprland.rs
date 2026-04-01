use serde::Deserialize;
use std::env;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HyprError {
    #[error("Hyprland instance signature not found. Is Hyprland running?")]
    NoInstance,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize, Clone)]
pub struct Workspace {
    id: i32,
    monitor: String,
}

impl Workspace {
    pub fn new(id: i32, monitor: String) -> Self {
        Self { id, monitor }
    }

    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn monitor(&self) -> &str {
        &self.monitor
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Monitor {
    name: String,
    #[serde(rename = "activeWorkspace")]
    active_workspace: ActiveWorkspace,
    focused: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ActiveWorkspace {
    id: i32,
}

impl Monitor {
    pub fn new(name: String, active_workspace_id: i32, focused: bool) -> Self {
        Self {
            name,
            active_workspace: ActiveWorkspace {
                id: active_workspace_id,
            },
            focused,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn active_workspace_id(&self) -> i32 {
        self.active_workspace.id
    }

    pub fn focused(&self) -> bool {
        self.focused
    }
}

#[cfg_attr(test, mockall::automock)]
pub trait HyprlandProvider: Send + Sync {
    fn get_monitors(&self) -> Result<Vec<Monitor>, HyprError>;
    fn get_workspaces(&self) -> Result<Vec<Workspace>, HyprError>;
}

pub struct RealHyprlandProvider;

impl HyprlandProvider for RealHyprlandProvider {
    fn get_monitors(&self) -> Result<Vec<Monitor>, HyprError> {
        get_monitors()
    }

    fn get_workspaces(&self) -> Result<Vec<Workspace>, HyprError> {
        get_workspaces()
    }
}

pub fn get_monitors() -> Result<Vec<Monitor>, HyprError> {
    let signature = env::var("HYPRLAND_INSTANCE_SIGNATURE").map_err(|_| HyprError::NoInstance)?;
    let xdg_runtime_dir = env::var("XDG_RUNTIME_DIR").map_err(|_| HyprError::NoInstance)?;

    let socket_path = PathBuf::from(xdg_runtime_dir)
        .join("hypr")
        .join(signature)
        .join(".socket.sock");

    let mut stream = UnixStream::connect(socket_path)?;
    stream.write_all(b"j/monitors")?;

    let mut response = String::new();
    stream.read_to_string(&mut response)?;

    let monitors: Vec<Monitor> = serde_json::from_str(&response)?;
    Ok(monitors)
}

pub fn get_workspaces() -> Result<Vec<Workspace>, HyprError> {
    let signature = env::var("HYPRLAND_INSTANCE_SIGNATURE").map_err(|_| HyprError::NoInstance)?;
    let xdg_runtime_dir = env::var("XDG_RUNTIME_DIR").map_err(|_| HyprError::NoInstance)?;

    let socket_path = PathBuf::from(xdg_runtime_dir)
        .join("hypr")
        .join(signature)
        .join(".socket.sock");

    let mut stream = UnixStream::connect(socket_path)?;
    stream.write_all(b"j/workspaces")?;

    let mut response = String::new();
    stream.read_to_string(&mut response)?;

    let workspaces: Vec<Workspace> = serde_json::from_str(&response)?;
    Ok(workspaces)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_getters() {
        let ws = Workspace::new(1, "eDP-1".to_string());
        assert_eq!(ws.id(), 1);
        assert_eq!(ws.monitor(), "eDP-1");
    }

    #[test]
    fn test_monitor_getters() {
        let m = Monitor::new("eDP-1".to_string(), 1, true);
        assert_eq!(m.name(), "eDP-1");
        assert_eq!(m.active_workspace_id(), 1);
        assert!(m.focused());
    }

    #[test]
    fn test_hypr_error_display() {
        let err = HyprError::NoInstance;
        assert_eq!(
            format!("{}", err),
            "Hyprland instance signature not found. Is Hyprland running?"
        );

        let err = HyprError::Io(std::io::Error::new(std::io::ErrorKind::Other, "test"));
        assert!(format!("{}", err).contains("IO error: test"));
    }

    #[test]
    fn test_real_provider_paths() {
        let provider = RealHyprlandProvider;
        // These will likely fail in test env, but we want to exercise the wrapper logic
        let _ = provider.get_monitors();
        let _ = provider.get_workspaces();
    }

    #[test]
    fn test_hypr_error_from_serde() {
        let json = "{ invalid";
        let res: std::result::Result<Workspace, serde_json::Error> = serde_json::from_str(json);
        let err: HyprError = res.unwrap_err().into();
        assert!(format!("{}", err).contains("JSON error"));
    }
}
