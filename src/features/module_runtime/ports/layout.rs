use crate::shared::primitives::geometry::{Rect, Size};
use crate::shared::primitives::{ChildModuleLayout, ModuleId, ModuleKey, MonitorId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutEvent {
    ModuleSizeChanged {
        monitor_id: MonitorId,
        module_id: ModuleId,
        size: Size,
    },
    ChildModuleSizeChanged {
        parent_id: ModuleId,
        child_key: ModuleKey,
        monitor_id: MonitorId,
        size: Size,
    },
    ContainerLayoutsCalculated {
        parent_id: ModuleId,
        monitor_id: MonitorId,
        layouts: Vec<ChildModuleLayout>,
    },
}

pub trait LayoutEventSender: Send + Sync {
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
        if let Err(e) = self.try_send(event) {
            tracing::error!(?e, "failed to send layout event via tokio channel");
        }
    }
}

impl LayoutEventSender for std::sync::mpsc::Sender<LayoutEvent> {
    fn send_layout_event(&self, event: LayoutEvent) {
        if let Err(e) = self.send(event) {
            tracing::error!(?e, "failed to send layout event via std channel");
        }
    }
}

pub trait LayoutSender: Send + Sync {
    fn send_layout(&self, layout: std::collections::HashMap<MonitorId, Rect>);
}
