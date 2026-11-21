//! The events that wake a module's event loop, merged from its signal, layout and pointer
//! channels.
use crate::shared::events::signals::SignalKind;
use crate::shared::primitives::MonitorId;

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum EventLoopEvent {
    Signals(Vec<SignalKind>),
    ModuleSizesChanged,
    LayoutChanged,
    Interaction(MonitorId, crate::shared::events::core::InteractionEvent),
    Shutdown,
}
