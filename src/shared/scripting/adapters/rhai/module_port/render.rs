//! The steps of a Rhai module's render: the monitor it gets, and the node its output becomes,
//! or the empty node when that fails.
use crate::features::vdom::domain::{ElementSelectors, Interactions, VNode};
use crate::shared::primitives::{MonitorId, ScriptMonitorInfo};
use crate::shared::scripting::adapters::rhai::module_impl::RhaiModule;
use rhai::{Dynamic, EvalAltResult};

impl RhaiModule {
    /// The monitor the last refresh listed with the id `monitor`, or one built from the id when it
    /// listed none.
    pub(super) fn monitor_for_render(&self, monitor: &MonitorId) -> ScriptMonitorInfo {
        self.cached_monitors
            .iter()
            .find(|monitor_info| monitor_info.id() == monitor)
            .cloned()
            .unwrap_or_else(|| ScriptMonitorInfo::from_id(monitor))
    }
}

/// The node the script's `render` returned, or an empty flex, logged, when the call failed or
/// returned something that is no node.
pub(super) fn node_from_render_output(output: Result<Dynamic, Box<EvalAltResult>>) -> VNode {
    match output {
        Ok(result) => rhai::serde::from_dynamic::<VNode>(&result).unwrap_or_else(|error| {
            super::log_render_output_rejected(&error);
            empty_flex()
        }),
        Err(error) => {
            super::log_render_failed(&error);
            empty_flex()
        }
    }
}

/// The node a module renders when its script produced none: an empty flex.
fn empty_flex() -> VNode {
    VNode::new_flex(vec![], ElementSelectors::default(), Interactions::default())
}
