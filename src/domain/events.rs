use crate::domain::workspace::{MonitorName, WorkspaceId, WorkspaceName};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WindowAddress(String);
impl WindowAddress { pub fn new(addr: impl Into<String>) -> Self { Self(addr.into()) } }

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WindowTitle(String);
impl WindowTitle { pub fn new(title: impl Into<String>) -> Self { Self(title.into()) } }

#[derive(Debug, Clone, PartialEq)]
pub enum WindowManagerEvent {
    WorkspaceActivated { id: WorkspaceId, name: WorkspaceName },
    MonitorFocused { monitor_name: MonitorName, workspace_id: WorkspaceId },
    WorkspaceCreated { id: WorkspaceId, name: WorkspaceName },
    WorkspaceDestroyed { id: WorkspaceId, name: WorkspaceName },
    WorkspaceMoved { id: WorkspaceId, name: WorkspaceName, monitor_name: MonitorName },
    WorkspaceRenamed { id: WorkspaceId, new_name: WorkspaceName },
    SpecialWorkspaceActivated { id: Option<WorkspaceId>, name: Option<WorkspaceName>, monitor_name: MonitorName },
    ActiveWindowChanged { address: WindowAddress },
    WindowTitleChanged { address: WindowAddress, title: WindowTitle },
}

#[derive(Debug, Clone, PartialEq)]
pub enum PointerEvent {
    PointerEnter,
    PointerLeave,
    PointerMotion { x: f64, y: f64 },
    Click { button: u32, x: f64, y: f64 },
    Scroll { axis: u32, amount: f64 },
}

pub type PointerSender = tokio::sync::broadcast::Sender<(crate::domain::ModuleId, PointerEvent)>;
pub type PointerReceiver = tokio::sync::broadcast::Receiver<(crate::domain::ModuleId, PointerEvent)>;
