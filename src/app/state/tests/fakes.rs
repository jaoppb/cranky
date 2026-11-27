//! A display server and a systray host that write down what the app asks of them.

use crate::features::layout_engine::domain::{FloatingKind, ShowFloating};
use crate::features::module_runtime::ports::LayoutSender;
use crate::features::systray::domain::{SystrayActionCall, SystrayId};
use crate::features::systray::ports::{SniPort, SniPortError};
use crate::shared::primitives::ModuleId;
use crate::shared::wayland::ports::{AppReadModel, DisplayServerError, DisplayServerPort};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// How long `RecordingDisplay::wait_for_events` waits before it ends `run`: long enough for the
/// loop to drain whatever a test queued, since every queued wakeup is ready at once.
const IDLE: Duration = Duration::from_millis(200);

/// Whether a fake grants what it is asked.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Answer {
    Grants,
    Refuses,
}

/// What a `RecordingDisplay` was asked, in order.
pub(super) type CallLog = Arc<Mutex<Vec<String>>>;

/// A display server that writes down every floating-surface request, dispatch and render it
/// gets, in order. Its `wait_for_events` fails once the loop has been idle for `IDLE`, which
/// ends `CrankyApp::run`; built `failing`, its floating-surface requests and its dispatch fail,
/// and its `wait_for_events` wakes at once so that the dispatch is reached.
pub(super) struct RecordingDisplay {
    calls: CallLog,
    answer: Answer,
}

impl RecordingDisplay {
    /// A display server whose every request succeeds.
    pub(super) fn new() -> Self {
        Self {
            calls: Arc::default(),
            answer: Answer::Grants,
        }
    }

    /// A display server whose floating-surface requests and dispatch fail.
    pub(super) fn failing() -> Self {
        Self {
            calls: Arc::default(),
            answer: Answer::Refuses,
        }
    }

    /// A handle on the requests this server records.
    pub(super) fn calls(&self) -> CallLog {
        self.calls.clone()
    }

    fn record(&self, call: String) {
        self.calls.lock().unwrap().push(call);
    }

    fn outcome(&self) -> Result<(), DisplayServerError> {
        if self.answer == Answer::Refuses {
            Err(DisplayServerError::Internal("no compositor".to_string()))
        } else {
            Ok(())
        }
    }
}

#[async_trait]
impl DisplayServerPort for RecordingDisplay {
    async fn wait_for_events(&mut self) -> Result<(), DisplayServerError> {
        if self.answer == Answer::Refuses {
            return Ok(());
        }
        tokio::time::sleep(IDLE).await;
        Err(DisplayServerError::Internal("idle".to_string()))
    }

    fn dispatch_pending(&mut self) -> Result<(), DisplayServerError> {
        self.record("dispatch".to_string());
        self.outcome()
    }

    fn flush(&mut self) -> Result<(), DisplayServerError> {
        Ok(())
    }

    fn render_all<Snd: LayoutSender + 'static>(
        &mut self,
        _read_model: &AppReadModel,
        _layout_senders: &HashMap<ModuleId, Snd>,
    ) -> Result<(), DisplayServerError> {
        self.record("render".to_string());
        Ok(())
    }

    fn show_floating_surface(&mut self, floating: ShowFloating) -> Result<(), DisplayServerError> {
        self.record(format!("show {:?}", floating.kind));
        self.outcome()
    }

    fn hide_floating_surface(&mut self, kind: &FloatingKind) -> Result<(), DisplayServerError> {
        self.record(format!("hide {kind:?}"));
        self.outcome()
    }
}

/// A systray host that writes down every action it is asked to trigger, as
/// `<item id> <action call>`, and fails each one when built `failing`.
pub(super) struct RecordingSni {
    calls: CallLog,
    answer: Answer,
}

impl RecordingSni {
    /// A host whose every action succeeds.
    pub(super) fn new() -> Self {
        Self {
            calls: Arc::default(),
            answer: Answer::Grants,
        }
    }

    /// A host whose every action fails.
    pub(super) fn failing() -> Self {
        Self {
            calls: Arc::default(),
            answer: Answer::Refuses,
        }
    }

    /// A handle on the actions this host records.
    pub(super) fn calls(&self) -> CallLog {
        self.calls.clone()
    }
}

#[async_trait]
impl SniPort for RecordingSni {
    async fn start(&mut self) -> Result<(), SniPortError> {
        Ok(())
    }

    async fn trigger_action(
        &self,
        id: &SystrayId,
        call: &SystrayActionCall,
    ) -> Result<(), SniPortError> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("{} {call:?}", id.as_str()));
        if self.answer == Answer::Refuses {
            return Err(SniPortError::ActionFailed {
                id: id.as_str().to_string(),
                error: "item gone".to_string(),
            });
        }
        Ok(())
    }
}
