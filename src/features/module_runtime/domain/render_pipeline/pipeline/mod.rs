mod phases;

use super::embed_state::EmbedState;
use super::floating_phase::{FloatingCache, FloatingContentCaches};
use crate::features::layout_engine::domain::RenderNode;
use crate::features::vdom::domain::{InteractionContext, VNode};
use crate::shared::primitives::geometry::{Rect, Size};
use crate::shared::primitives::{ChildSizesMap, MonitorId};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default)]
pub struct RenderPipeline {
    sizes: HashMap<MonitorId, Size>,
    pub(super) embed: EmbedState,
    rendered_bounds: HashMap<MonitorId, Rect>,
    render_trees: HashMap<MonitorId, RenderNode>,
    vdom_trees: HashMap<MonitorId, VNode>,
    last_child_sizes: HashMap<MonitorId, Option<ChildSizesMap>>,
    last_interactions: HashMap<MonitorId, Option<InteractionContext>>,
    /// Last `(StyledNode, RenderNode)` per floating kind — lets
    /// `layout_floating` skip re-laying-out unchanged floating content.
    popup_content_cache: FloatingCache,
    panel_content_cache: FloatingCache,
    tooltip_content_cache: FloatingCache,
}

impl RenderPipeline {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn sizes(&self) -> &HashMap<MonitorId, Size> {
        &self.sizes
    }

    pub const fn sizes_mut(&mut self) -> &mut HashMap<MonitorId, Size> {
        &mut self.sizes
    }

    #[must_use]
    pub const fn rendered_bounds(&self) -> &HashMap<MonitorId, Rect> {
        &self.rendered_bounds
    }

    #[must_use]
    pub const fn render_trees(&self) -> &HashMap<MonitorId, RenderNode> {
        &self.render_trees
    }

    pub const fn render_trees_mut(&mut self) -> &mut HashMap<MonitorId, RenderNode> {
        &mut self.render_trees
    }

    #[must_use]
    pub const fn vdom_trees(&self) -> &HashMap<MonitorId, VNode> {
        &self.vdom_trees
    }

    pub const fn vdom_trees_mut(&mut self) -> &mut HashMap<MonitorId, VNode> {
        &mut self.vdom_trees
    }

    #[must_use]
    pub const fn last_child_sizes(&self) -> &HashMap<MonitorId, Option<ChildSizesMap>> {
        &self.last_child_sizes
    }

    pub const fn last_child_sizes_mut(&mut self) -> &mut HashMap<MonitorId, Option<ChildSizesMap>> {
        &mut self.last_child_sizes
    }

    #[must_use]
    pub const fn last_interactions(&self) -> &HashMap<MonitorId, Option<InteractionContext>> {
        &self.last_interactions
    }

    pub const fn last_interactions_mut(
        &mut self,
    ) -> &mut HashMap<MonitorId, Option<InteractionContext>> {
        &mut self.last_interactions
    }

    /// Disjoint mutable borrows of all three floating-content caches at once.
    pub const fn floating_content_caches_mut(&mut self) -> FloatingContentCaches<'_> {
        FloatingContentCaches {
            popup: &mut self.popup_content_cache,
            panel: &mut self.panel_content_cache,
            tooltip: &mut self.tooltip_content_cache,
        }
    }

    /// Drops every per-monitor entry for a monitor not in `live` — nothing
    /// else here ever removes an entry, so a disconnected monitor would
    /// otherwise keep its render state alive forever.
    pub fn retain_monitors(&mut self, live: &HashSet<MonitorId>) {
        self.sizes.retain(|id, _| live.contains(id));
        self.embed.retain_monitors(live);
        self.rendered_bounds.retain(|id, _| live.contains(id));
        self.render_trees.retain(|id, _| live.contains(id));
        self.vdom_trees.retain(|id, _| live.contains(id));
        self.last_child_sizes.retain(|id, _| live.contains(id));
        self.last_interactions.retain(|id, _| live.contains(id));
        self.popup_content_cache.retain(|id, _| live.contains(id));
        self.panel_content_cache.retain(|id, _| live.contains(id));
        self.tooltip_content_cache.retain(|id, _| live.contains(id));
    }
}
