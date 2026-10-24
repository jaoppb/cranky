use crate::shared::primitives::MonitorId;
use std::collections::{HashMap, HashSet};

/// Per-monitor bookkeeping for a module embedded in someone else's tree.
#[derive(Debug, Default)]
pub struct EmbedState {
    /// Width the layout was last pinned to (`None`: unpinned).
    settled_widths: HashMap<MonitorId, Option<u32>>,
    /// Consecutive evaluations a floating surface was held back waiting for
    /// an embedded child to settle (see `floating_phase::gate`).
    gate_rounds: HashMap<MonitorId, u8>,
}

impl EmbedState {
    /// Records the width this monitor was laid out at; true when it differs
    /// from the last one recorded (or nothing was recorded yet).
    pub fn settle(&mut self, monitor_id: &MonitorId, width: Option<u32>) -> bool {
        self.settled_widths.insert(monitor_id.clone(), width) != Some(width)
    }

    pub const fn gate_rounds_mut(&mut self) -> &mut HashMap<MonitorId, u8> {
        &mut self.gate_rounds
    }

    pub fn retain_monitors(&mut self, live: &HashSet<MonitorId>) {
        self.settled_widths.retain(|id, _| live.contains(id));
        self.gate_rounds.retain(|id, _| live.contains(id));
    }
}
