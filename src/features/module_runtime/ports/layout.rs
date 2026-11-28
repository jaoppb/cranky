//! Layout messages between module actors and the app: the size and child-layout events modules send
//! up, and the per-monitor bounds sent back down.
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::{ChildBounds, ChildModuleLayout, ModuleId, MonitorId};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LayoutEvent {
    ModuleSizeChanged {
        monitor_id: MonitorId,
        module_id: ModuleId,
        /// Intrinsic size (laid out with no pin).
        size: Size,
        /// Width the module was laid out at once its embedder pinned one.
        settled_width: Option<u32>,
    },
    ContainerLayoutsCalculated {
        parent_id: ModuleId,
        monitor_id: MonitorId,
        layouts: Vec<ChildModuleLayout>,
    },
}

pub(crate) trait LayoutEventSender: Send + Sync {
    fn send_layout_event(&self, event: LayoutEvent);
}

impl<F> LayoutEventSender for F
where
    F: Fn(LayoutEvent) + Send + Sync,
{
    fn send_layout_event(&self, event: LayoutEvent) {
        self(event);
    }
}

impl LayoutEventSender for tokio::sync::mpsc::Sender<LayoutEvent> {
    fn send_layout_event(&self, event: LayoutEvent) {
        if let Err(error) = self.try_send(event) {
            log_tokio_send_failed(&error);
        }
    }
}

impl LayoutEventSender for std::sync::mpsc::Sender<LayoutEvent> {
    fn send_layout_event(&self, event: LayoutEvent) {
        if let Err(error) = self.send(event) {
            log_std_send_failed(&error);
        }
    }
}

/// Logs a layout event the tokio channel refused with `error`.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_tokio_send_failed(error: &tokio::sync::mpsc::error::TrySendError<LayoutEvent>) {
    tracing::error!(?error, "failed to send layout event via tokio channel");
}

/// Logs a layout event the std channel refused with `error`.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_std_send_failed(error: &std::sync::mpsc::SendError<LayoutEvent>) {
    tracing::error!(?error, "failed to send layout event via std channel");
}

/// Delivers the per-monitor bounds the app assigned to a module to that module's actor.
pub trait LayoutSender: Send + Sync {
    /// Replaces the module's bounds with `layout`, one entry per monitor it is shown on.
    fn send_layout(&self, layout: std::collections::HashMap<MonitorId, ChildBounds>);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::module_runtime::test_support::capture_logs;

    const TARGET: &str = "cranky::features::module_runtime::ports::layout";

    fn grown() -> LayoutEvent {
        LayoutEvent::ModuleSizeChanged {
            monitor_id: MonitorId::new("DP-1"),
            module_id: ModuleId::new(3),
            size: Size::new(40, 20),
            settled_width: None,
        }
    }

    /// A tokio channel delivers the event; one that is full drops it and logs why.
    #[test]
    fn a_tokio_channel_delivers_and_logs_a_full_channel() {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        let logs = capture_logs(|| {
            sender.send_layout_event(grown());
            sender.send_layout_event(grown());
        });
        assert_eq!(receiver.try_recv(), Ok(grown()));
        assert!(receiver.try_recv().is_err());
        assert!(logs.contains(&format!(
            "ERROR {TARGET}: failed to send layout event via tokio channel error=\"Full(..)\""
        )));
    }

    /// A std channel delivers the event; one whose receiver is gone drops it and logs why.
    #[test]
    fn a_std_channel_delivers_and_logs_a_closed_channel() {
        let (sender, receiver) = std::sync::mpsc::channel();
        sender.send_layout_event(grown());
        assert_eq!(receiver.try_recv(), Ok(grown()));
        drop(receiver);
        let logs = capture_logs(|| sender.send_layout_event(grown()));
        assert!(logs.contains(&format!(
            "ERROR {TARGET}: failed to send layout event via std channel error=SendError {{ .. }}"
        )));
    }
}
