//! Commands that modules and the Wayland adapter send to the app loop to render or to show and
//! hide floating surfaces.
use super::popup::FloatingKind;
use crate::shared::primitives::MonitorId;
use crate::shared::primitives::geometry::{Rect, Size};
use crate::shared::primitives::render::RenderBuffer;

/// Everything needed to show or update one floating surface.
///
/// Bundled so that adding `parent` (Phase 5 nesting) didn't push
/// `DisplayServerPort::show_floating_surface` past clippy's
/// `too_many_arguments` threshold. `logical_size` travels separately from
/// `buffer`'s own (physical, scaled) size because the positioner needs
/// logical units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowFloating {
    pub(crate) kind: FloatingKind,
    pub(crate) monitor_id: Option<MonitorId>,
    pub(crate) anchor_rect: Option<Rect>,
    pub(crate) buffer: RenderBuffer,
    pub(crate) logical_size: Size,
    pub(crate) offset: Option<crate::shared::primitives::PopupOffset>,
    /// The floating surface this one nests inside (decision 7), if any —
    /// the `FloatingKind` whose surface contains the subsurface of the
    /// module that owns this popup/panel. `None` for one owned by a module
    /// embedded directly in the bar tree.
    pub(crate) parent: Option<FloatingKind>,
}

/// Request from a module or the Wayland adapter to the app loop, which owns the display server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisplayCommand {
    /// Re-render every module; the Wayland adapter sends it after each non-zero bar configure
    /// (including resizes) and when an output is removed.
    RequestRender,
    /// The module has already laid out and painted this floating surface's
    /// content — Wayland's job is placing and blitting a buffer, never
    /// computing one.
    ShowFloatingSurface(ShowFloating),
    /// Unmap the floating surface identified by `kind`.
    HideFloatingSurface {
        /// Which floating surface to hide.
        kind: FloatingKind,
    },
}

pub(crate) trait DisplayCommandSender: Send + Sync {
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
        if let Err(error) = self.try_send(cmd) {
            log_tokio_send_failed(&error);
        }
    }
}

impl DisplayCommandSender for std::sync::mpsc::Sender<DisplayCommand> {
    fn send_display_command(&self, cmd: DisplayCommand) {
        if let Err(error) = self.send(cmd) {
            log_std_send_failed(&error);
        }
    }
}

/// Logs that the tokio channel refused a display command.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_tokio_send_failed(error: &tokio::sync::mpsc::error::TrySendError<DisplayCommand>) {
    tracing::error!(?error, "failed to send display command via tokio channel");
}

/// Logs that the std channel refused a display command.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_std_send_failed(error: &std::sync::mpsc::SendError<DisplayCommand>) {
    tracing::error!(?error, "failed to send display command via std channel");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::module_runtime::test_support::capture_logs;

    const TARGET: &str = "cranky::features::layout_engine::domain::commands";

    fn hide_tooltip() -> DisplayCommand {
        DisplayCommand::HideFloatingSurface {
            kind: FloatingKind::Tooltip,
        }
    }

    /// A closure is handed every command.
    #[test]
    fn a_closure_receives_the_command() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let forward = move |cmd| sender.send(cmd).unwrap();
        forward.send_display_command(hide_tooltip());
        assert_eq!(receiver.try_recv(), Ok(hide_tooltip()));
    }

    /// A tokio channel delivers the command; one that is full drops it and logs why.
    #[test]
    fn a_tokio_channel_delivers_and_logs_a_full_channel() {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        let logs = capture_logs(|| {
            sender.send_display_command(hide_tooltip());
            sender.send_display_command(hide_tooltip());
        });
        assert_eq!(receiver.try_recv(), Ok(hide_tooltip()));
        assert!(receiver.try_recv().is_err());
        assert!(logs.contains(&format!(
            "ERROR {TARGET}: failed to send display command via tokio channel error=\"Full(..)\""
        )));
    }

    /// A std channel delivers the command; one whose receiver is gone drops it and logs why.
    #[test]
    fn a_std_channel_delivers_and_logs_a_closed_channel() {
        let (sender, receiver) = std::sync::mpsc::channel();
        sender.send_display_command(hide_tooltip());
        assert_eq!(receiver.try_recv(), Ok(hide_tooltip()));
        drop(receiver);
        let logs = capture_logs(|| sender.send_display_command(hide_tooltip()));
        assert!(logs.contains(&format!(
            "ERROR {TARGET}: failed to send display command via std channel error=SendError {{ .. }}"
        )));
    }
}
