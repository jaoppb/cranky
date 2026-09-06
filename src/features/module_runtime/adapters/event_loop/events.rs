use crate::shared::events::signals::SignalKind;
use crate::shared::primitives::MonitorId;

#[derive(Debug, PartialEq, Clone)]
pub enum EventLoopEvent {
    Signals(Vec<SignalKind>),
    ModuleSizesChanged,
    LayoutChanged,
    Pointer(MonitorId, crate::shared::events::core::PointerEvent),
    Shutdown,
}
