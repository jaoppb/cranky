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

#[cfg(test)]
mod tests {
    use super::{EventLoopEvent, Interaction, input_event};
    use crate::features::module_runtime::test_support::capture_logs;
    use crate::shared::events::core::{InteractionEvent, PointerEvent, SurfaceKind};
    use crate::shared::primitives::geometry::Position;
    use crate::shared::primitives::{ModuleId, MonitorId};
    use tokio::sync::broadcast::error::RecvError;

    const MODULE: ModuleId = ModuleId::new(42);
    const TARGET: &str = "cranky::features::module_runtime::adapters::event_loop::poll";

    /// A pointer motion on DP-1's bar, for module `id`.
    fn motion_for(id: ModuleId) -> Interaction {
        let event = InteractionEvent::Pointer(PointerEvent::PointerMotion {
            surface: SurfaceKind::Bar,
            pos: Position::new(3, 4),
        });
        (id, MonitorId::new("DP-1"), event)
    }

    /// The event `res` yields for module 42, and what it logged.
    fn yielded(res: Result<Interaction, RecvError>) -> (EventLoopEvent, String) {
        let mut event = None;
        let logs = capture_logs(|| event = Some(input_event(MODULE, res)));
        (event.expect("input_event ran"), logs)
    }

    #[test]
    fn an_interaction_for_this_module_is_yielded_and_one_for_another_is_no_signal() {
        let (_, monitor, event) = motion_for(MODULE);
        let expected = EventLoopEvent::Interaction(monitor, event);
        assert_eq!(yielded(Ok(motion_for(MODULE))).0, expected);
        let other = yielded(Ok(motion_for(ModuleId::new(7))));
        assert_eq!(other, (EventLoopEvent::Signals(Vec::new()), String::new()));
    }

    #[test]
    fn skipped_interactions_are_logged_and_yield_no_signal() {
        let (event, logs) = yielded(Err(RecvError::Lagged(3)));
        assert_eq!(event, EventLoopEvent::Signals(Vec::new()));
        let line = format!(
            "WARN {TARGET}: ModuleActor input_rx lagged, skipped messages module=42 lagged=3\n"
        );
        assert_eq!(logs.trim_start(), line);
    }

    #[test]
    fn a_closed_channel_is_logged_and_shuts_the_loop_down() {
        let (event, logs) = yielded(Err(RecvError::Closed));
        assert_eq!(event, EventLoopEvent::Shutdown);
        let line = format!("DEBUG {TARGET}: ModuleActor input_rx closed module=42\n");
        assert_eq!(logs, line);
    }
}
