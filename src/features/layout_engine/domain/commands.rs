use super::popup::FloatingKind;
use crate::shared::primitives::geometry::{Rect, Size};
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::primitives::MonitorId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisplayCommand {
    RequestRender,
    /// The module has already laid out and painted this floating surface's
    /// content — Wayland's job is placing and blitting a buffer, never
    /// computing one. `logical_size` travels separately from `buffer`'s own
    /// (physical, scaled) size because the positioner needs logical units.
    ShowFloatingSurface {
        kind: FloatingKind,
        monitor_id: Option<MonitorId>,
        anchor_rect: Option<Rect>,
        buffer: RenderBuffer,
        logical_size: Size,
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
