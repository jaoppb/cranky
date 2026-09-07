use crate::shared::events::signals::SignalKind;
use crate::shared::primitives::MonitorId;

#[derive(Debug, PartialEq, Clone)]
pub enum EventLoopEvent {
    Signals(Vec<SignalKind>),
    ModuleSizesChanged,
    LayoutChanged,
    Interaction(MonitorId, crate::shared::events::core::InteractionEvent),
    Shutdown,
}
