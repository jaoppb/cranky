use super::popup::FloatingKind;
use super::styled_node::StyledNode;
use crate::shared::primitives::geometry::Rect;
use crate::shared::primitives::MonitorId;

#[derive(Debug, Clone, PartialEq)]
pub enum DisplayCommand {
    RequestRender,
    ShowFloatingSurface {
        kind: FloatingKind,
        monitor_id: Option<MonitorId>,
        anchor_rect: Option<Rect>,
        layout: Box<StyledNode>,
        offset: Option<crate::shared::primitives::PopupOffset>,
    },
    HideFloatingSurface {
        kind: FloatingKind,
    },
}

pub trait DisplayCommandSender: Send + Sync {
    fn send_display_command(&self, cmd: DisplayCommand);
}

impl<F> DisplayCommandSender for F
where
    F: Fn(DisplayCommand) + Send + Sync,
{
    fn send_display_command(&self, cmd: DisplayCommand) {
        self(cmd);
    }
}

impl DisplayCommandSender for tokio::sync::mpsc::Sender<DisplayCommand> {
    fn send_display_command(&self, cmd: DisplayCommand) {
        if let Err(e) = self.try_send(cmd) {
            tracing::error!(?e, "failed to send display command via tokio channel");
        }
    }
}

impl DisplayCommandSender for std::sync::mpsc::Sender<DisplayCommand> {
    fn send_display_command(&self, cmd: DisplayCommand) {
        if let Err(e) = self.send(cmd) {
            tracing::error!(?e, "failed to send display command via std channel");
        }
    }
}
