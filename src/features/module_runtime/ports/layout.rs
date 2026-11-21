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
            tracing::error!(?error, "failed to send layout event via tokio channel");
        }
    }
}

impl LayoutEventSender for std::sync::mpsc::Sender<LayoutEvent> {
    fn send_layout_event(&self, event: LayoutEvent) {
        if let Err(error) = self.send(event) {
            tracing::error!(?error, "failed to send layout event via std channel");
        }
    }
}

/// Delivers the per-monitor bounds the app assigned to a module to that module's actor.
pub trait LayoutSender: Send + Sync {
    /// Replaces the module's bounds with `layout`, one entry per monitor it is shown on.
    fn send_layout(&self, layout: std::collections::HashMap<MonitorId, ChildBounds>);
}
