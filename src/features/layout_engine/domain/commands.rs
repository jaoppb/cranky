use super::popup::FloatingKind;
use crate::shared::primitives::geometry::{Rect, Size};
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::primitives::MonitorId;

/// Everything needed to show or update one floating surface.
///
/// Bundled so that adding `parent` (Phase 5 nesting) didn't push
/// `DisplayServerPort::show_floating_surface` past clippy's
/// `too_many_arguments` threshold. `logical_size` travels separately from
/// `buffer`'s own (physical, scaled) size because the positioner needs
/// logical units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowFloating {
    pub kind: FloatingKind,
    pub monitor_id: Option<MonitorId>,
    pub anchor_rect: Option<Rect>,
    pub buffer: RenderBuffer,
    pub logical_size: Size,
    pub offset: Option<crate::shared::primitives::PopupOffset>,
    /// The floating surface this one nests inside (decision 7), if any —
    /// the `FloatingKind` whose surface contains the subsurface of the
    /// module that owns this popup/panel. `None` for one owned by a module
    /// embedded directly in the bar tree.
    pub parent: Option<FloatingKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisplayCommand {
    RequestRender,
    /// The module has already laid out and painted this floating surface's
    /// content — Wayland's job is placing and blitting a buffer, never
    /// computing one.
    ShowFloatingSurface(ShowFloating),
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
