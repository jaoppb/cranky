//! A run loop spawned on a current-thread runtime, with the log of its port, for the tests that
//! drive it through the hub.
use super::fixture::{Wiring, hub_with_dp1, wired_event_loop};
use crate::features::module_runtime::test_support::{PortLog, RecordingModulePort};
use crate::features::vdom::domain::{
    ClickHandlers, ElementSelectors, Interactions, TextContent, UiAction, VNode,
};
use crate::shared::events::core::{InteractionEvent, PointerButton, PointerEvent, SurfaceKind};
use crate::shared::events::signals::SignalKind;
use crate::shared::primitives::geometry::Position;
use crate::shared::primitives::{FunctionName, ModuleId, MonitorId};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

pub(super) const SUBS: [SignalKind; 2] = [SignalKind::Time, SignalKind::Hyprland];

/// A 10x10 bar text that calls `toggle` when clicked.
pub(super) fn clickable() -> VNode {
    let toggle = ClickHandlers::from_single(UiAction::ScriptCall(FunctionName::new("toggle")));
    VNode::new_text(
        TextContent::new("bar".to_string()),
        ElementSelectors::default(),
        Interactions::new(Some(toggle), None, None),
    )
}

/// A run loop spawned for `MODULE`, with the log of its port.
pub(super) struct Running {
    pub(super) wiring: Wiring,
    log: Arc<Mutex<PortLog>>,
    pub(super) task: JoinHandle<()>,
}

impl Running {
    /// What the port was asked to do since the last call.
    pub(super) fn take(&self) -> PortLog {
        std::mem::take(&mut *self.log.lock().unwrap())
    }

    /// Sends `event` on DP-1 to `module` through the hub's pointer channel.
    pub(super) fn interact(&self, module: ModuleId, event: InteractionEvent) {
        let _ = self
            .wiring
            .hub
            .pointer_tx()
            .send((module, MonitorId::new("DP-1"), event));
    }
}

/// Lets the spawned run loop handle everything sent so far.
pub(super) async fn settle() {
    for _ in 0..50 {
        tokio::task::yield_now().await;
    }
}

/// Spawns the run loop of `port` on a hub with DP-1, and lets it start.
pub(super) async fn start(port: RecordingModulePort) -> Running {
    let log = port.log();
    let (event_loop, wiring) = wired_event_loop(port, hub_with_dp1(1.0));
    let task = tokio::spawn(event_loop.run());
    settle().await;
    Running { wiring, log, task }
}

/// A left click at (5, 5) on the bar.
pub(super) fn click() -> InteractionEvent {
    InteractionEvent::Pointer(PointerEvent::Click {
        surface: SurfaceKind::Bar,
        button: PointerButton::Left,
        pos: Position::new(5, 5),
    })
}

/// Each refresh's signals, in any order.
pub(super) fn kinds(refreshes: &[Vec<SignalKind>]) -> Vec<HashSet<SignalKind>> {
    refreshes
        .iter()
        .map(|signals| signals.iter().copied().collect())
        .collect()
}
