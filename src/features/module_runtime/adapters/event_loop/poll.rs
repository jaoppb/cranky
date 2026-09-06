use super::events::EventLoopEvent;
use super::runner::EventLoop;
use crate::features::layout_engine::domain::DisplayCommandSender;
use crate::features::module_runtime::ports::LayoutEventSender;
use crate::features::vdom::domain::UiCommandSender;
use crate::shared::events::signals::SignalKind;
use crate::shared::primitives::MonitorId;
use crate::shared::rendering::ports::canvas::CanvasFactory;
use futures_util::StreamExt;
use std::collections::{HashMap, HashSet};

impl<
    F: CanvasFactory + 'static,
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
> EventLoop<F, LS, DS, US>
{
    pub async fn poll_next_event(
        &mut self,
        events_stream: &mut futures_util::stream::SelectAll<
            futures_util::stream::BoxStream<'static, SignalKind>,
        >,
        module_sizes_rx: &mut tokio::sync::watch::Receiver<
            HashMap<MonitorId, crate::shared::primitives::ChildSizesMap>,
        >,
    ) -> EventLoopEvent {
        let ctx_id = self.ctx.id();
        let (layout_rx, input_rx) = self.ctx.rxs_mut();

        tokio::select! {
            Some(sig) = events_stream.next(), if !events_stream.is_empty() => {
                let mut changed_signals = HashSet::new();
                changed_signals.insert(sig);
                while let Some(Some(sig2)) = futures_util::FutureExt::now_or_never(events_stream.next()) {
                    changed_signals.insert(sig2);
                }
                EventLoopEvent::Signals(changed_signals.into_iter().collect())
            }
            res = module_sizes_rx.changed() => {
                if res.is_err() {
                    EventLoopEvent::Shutdown
                } else {
                    EventLoopEvent::ModuleSizesChanged
                }
            }
            res = layout_rx.changed() => {
                if res.is_err() {
                    EventLoopEvent::Shutdown
                } else {
                    EventLoopEvent::LayoutChanged
                }
            }
            res = input_rx.recv() => {
                match res {
                    Ok((target_id, monitor_id, event)) => {
                        if target_id == ctx_id {
                            EventLoopEvent::Pointer(monitor_id, event)
                        } else {
                            EventLoopEvent::Signals(Vec::new())
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(module = %ctx_id, lagged = n, "ModuleActor input_rx lagged, skipped messages");
                        EventLoopEvent::Signals(Vec::new())
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::debug!(module = %ctx_id, "ModuleActor input_rx closed");
                        EventLoopEvent::Shutdown
                    }
                }
            }
        }
    }
}
