use super::events::EventLoopEvent;
use super::runner::EventLoop;
use crate::features::layout_engine::domain::DisplayCommandSender;
use crate::features::module_runtime::ports::LayoutEventSender;
use crate::features::vdom::domain::UiCommandSender;
use crate::shared::events::core::InteractionEvent;
use crate::shared::events::signals::SignalKind;
use crate::shared::primitives::ModuleId;
use crate::shared::primitives::MonitorId;
use crate::shared::rendering::ports::canvas::CanvasFactory;
use futures_util::StreamExt;
use futures_util::stream::{BoxStream, SelectAll};
use std::collections::{HashMap, HashSet};
use tokio::sync::broadcast;
use tokio::sync::watch::error::RecvError;

/// What the interaction broadcast carries: the module it is for, the monitor and the event.
type Interaction = (ModuleId, MonitorId, InteractionEvent);

impl<
    F: CanvasFactory + 'static,
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
> EventLoop<F, LS, DS, US>
{
    pub(crate) async fn poll_next_event(
        &mut self,
        events_stream: &mut SelectAll<BoxStream<'static, SignalKind>>,
        module_sizes_rx: &mut tokio::sync::watch::Receiver<
            HashMap<MonitorId, crate::shared::primitives::ChildSizesMap>,
        >,
    ) -> EventLoopEvent {
        let ctx_id = self.ctx.id();
        let (layout_rx, input_rx) = self.ctx.rxs_mut();

        tokio::select! {
            Some(sig) = events_stream.next(), if !events_stream.is_empty() => {
                drain_signals(sig, events_stream)
            }
            res = module_sizes_rx.changed() => watch_event(res, EventLoopEvent::ModuleSizesChanged),
            res = layout_rx.changed() => watch_event(res, EventLoopEvent::LayoutChanged),
            res = input_rx.recv() => input_event(ctx_id, res),
        }
    }
}

/// The signals that changed: `first`, and every other one `events_stream` already has ready.
fn drain_signals(
    first: SignalKind,
    events_stream: &mut SelectAll<BoxStream<'static, SignalKind>>,
) -> EventLoopEvent {
    let mut changed_signals = HashSet::new();
    changed_signals.insert(first);
    while let Some(Some(sig2)) = futures_util::FutureExt::now_or_never(events_stream.next()) {
        changed_signals.insert(sig2);
    }
    EventLoopEvent::Signals(changed_signals.into_iter().collect())
}

/// `event` when a watch reported a change, shutdown when its sender is gone.
fn watch_event(res: Result<(), RecvError>, event: EventLoopEvent) -> EventLoopEvent {
    res.map_or(EventLoopEvent::Shutdown, |()| event)
}

/// The event an interaction broadcast yields for module `ctx_id`: the interaction when it is for
/// this module, no signal when it is for another or messages were skipped, shutdown once the
/// channel closed.
fn input_event(
    ctx_id: ModuleId,
    res: Result<Interaction, broadcast::error::RecvError>,
) -> EventLoopEvent {
    match res {
        Ok((target_id, monitor_id, event)) if target_id == ctx_id => {
            EventLoopEvent::Interaction(monitor_id, event)
        }
        Ok(_) => EventLoopEvent::Signals(Vec::new()),
        Err(broadcast::error::RecvError::Lagged(count)) => {
            log_input_lagged(ctx_id, count);
            EventLoopEvent::Signals(Vec::new())
        }
        Err(broadcast::error::RecvError::Closed) => {
            log_input_closed(ctx_id);
            EventLoopEvent::Shutdown
        }
    }
}

/// Logs that module `ctx_id` fell `count` interactions behind, which were skipped.
fn log_input_lagged(ctx_id: ModuleId, count: u64) {
    tracing::warn!(module = %ctx_id, lagged = count, "ModuleActor input_rx lagged, skipped messages");
}

/// Logs that the interaction channel of module `ctx_id` closed.
fn log_input_closed(ctx_id: ModuleId) {
    tracing::debug!(module = %ctx_id, "ModuleActor input_rx closed");
}
