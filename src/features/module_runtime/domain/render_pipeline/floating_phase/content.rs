//! The layout pass and per-monitor cache shared by all three floating kinds.
use super::super::measurer::ModuleSizeMeasurer;
use crate::features::layout_engine::domain::{LayoutError, LayoutRoot};
use crate::features::layout_engine::domain::{RenderNode, StyledNode};
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::vdom::domain::InteractionContext;
use crate::shared::primitives::SizeConstraint;
use crate::shared::primitives::geometry::{Position, Scale};
use crate::shared::primitives::{ChildSizesMap, MonitorId};
use crate::shared::rendering::ports::canvas::CanvasFactory;
use crate::shared::rendering::ports::font::Font;
use std::collections::HashMap;

/// Last laid-out floating content per monitor, with its child-size snapshot.
///
/// An embedded module's host box is measured from what the child reported,
/// so the same content under different reports is a different layout.
pub(crate) type FloatingCache = HashMap<MonitorId, (StyledNode, Option<ChildSizesMap>, RenderNode)>;

/// One floating kind's layout engine, with the cache of what it last laid out.
///
/// Each kind has its own of both, so laying out one never disturbs another's.
pub(crate) struct FloatingEngine<'a, E: LayoutEnginePort> {
    pub(crate) engine: &'a mut E,
    pub(crate) cache: &'a mut FloatingCache,
}

impl<'a, E: LayoutEnginePort> FloatingEngine<'a, E> {
    /// `engine` laying out one floating kind, with that kind's `cache`.
    pub(crate) const fn new(engine: &'a mut E, cache: &'a mut FloatingCache) -> Self {
        Self { engine, cache }
    }
}

/// What every floating kind is laid out with on one monitor during one render.
pub(crate) struct FloatingPass<'a, F: CanvasFactory> {
    pub(crate) monitor_id: &'a MonitorId,
    pub(crate) canvas_factory: &'a mut F,
    pub(crate) scale: Scale,
    pub(crate) current_child_sizes: Option<&'a ChildSizesMap>,
    /// The hovered, pressed and focused nodes on the monitor; the hovered one picks the tooltip.
    pub(crate) interaction: Option<&'a InteractionContext>,
}

/// Lays out one piece of floating content (a popup's, panel's or tooltip's)
/// with the pipeline's own measurer, scale and font fallback — shared by all
/// three so hit-testing and painting always agree, whichever kind it is.
pub(super) fn layout_floating_content<F: CanvasFactory, E: LayoutEnginePort>(
    layout_engine: &mut E,
    pass: &mut FloatingPass<'_, F>,
    content: &StyledNode,
) -> Option<RenderNode> {
    let measurer_inner = pass
        .canvas_factory
        .create_text_measurer(pass.scale, Font::fallback());
    let mut measurer = ModuleSizeMeasurer::new(measurer_inner, pass.current_child_sizes);
    match layout_engine.calculate_layout(
        LayoutRoot::new(content.clone(), Position::new(0, 0), SizeConstraint::none()),
        &mut measurer,
    ) {
        Ok(node) => Some(node),
        Err(error) => {
            log_layout_failed(&error);
            None
        }
    }
}

/// Logs a floating layout pass that failed with `error`.
fn log_layout_failed(error: &LayoutError) {
    tracing::error!(err = ?error, "Floating content layout calculation failed");
}

/// `layout_floating_content`, but skips the taffy pass entirely when
/// `content` compares equal to whatever was last laid out for this kind on
/// this monitor — a popup/panel/tooltip left open and unchanged otherwise
/// pays a full layout + clone on every render where anything else (a
/// sibling module's size, say) is what's actually dirty.
pub(super) fn layout_floating_content_cached<F: CanvasFactory, E: LayoutEnginePort>(
    engine: FloatingEngine<'_, E>,
    pass: &mut FloatingPass<'_, F>,
    content: &StyledNode,
) -> Option<RenderNode> {
    let FloatingEngine {
        engine: layout_engine,
        cache,
    } = engine;
    if let Some((cached_content, cached_sizes, cached_node)) = cache.get(pass.monitor_id)
        && cached_content == content
        && cached_sizes.as_ref() == pass.current_child_sizes
    {
        return Some(cached_node.clone());
    }
    let node = layout_floating_content(layout_engine, pass, content)?;
    cache.insert(
        pass.monitor_id.clone(),
        (
            content.clone(),
            pass.current_child_sizes.cloned(),
            node.clone(),
        ),
    );
    Some(node)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::layout_engine::domain::{NodePath, SurfaceSpace, TextMeasurer};
    use crate::features::module_runtime::test_support::MockCanvasFactory;
    use crate::features::styling::domain::ComputedStyle;
    use crate::shared::primitives::geometry::{Rect, Size};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Records how many times a layout pass actually ran, so the cache's
    /// "skip the taffy call on an unchanged node" behavior is directly
    /// observable instead of inferred from timing.
    struct CountingEngine {
        calls: Arc<AtomicUsize>,
    }

    impl LayoutEnginePort for CountingEngine {
        fn calculate_layout(
            &mut self,
            _root: LayoutRoot,
            _measurer: &mut dyn TextMeasurer,
        ) -> Result<RenderNode, LayoutError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(RenderNode::Rect {
                path: NodePath::root_in(SurfaceSpace::Tooltip),
                node_key: None,
                rect: Rect::new(Position::new(0, 0), Size::new(10, 10)),
                style: ComputedStyle::default(),
                on_click: None,
                on_hover: None,
                tooltip: None,
                popup: None,
                panel: None,
            })
        }
    }

    fn pass<'a>(
        monitor_id: &'a MonitorId,
        canvas_factory: &'a mut MockCanvasFactory,
        current_child_sizes: Option<&'a ChildSizesMap>,
    ) -> FloatingPass<'a, MockCanvasFactory> {
        FloatingPass {
            monitor_id,
            canvas_factory,
            scale: Scale::new(1.0),
            current_child_sizes,
            interaction: None,
        }
    }

    fn text_node(text: &str) -> StyledNode {
        StyledNode::Text {
            path: NodePath::root_in(SurfaceSpace::Tooltip),
            node_key: None,
            text: crate::features::vdom::domain::TextContent::new(text.to_string()),
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        }
    }

    /// A counting engine and its cache, laying content out on DP-1.
    struct CachedLayouts {
        calls: Arc<AtomicUsize>,
        engine: CountingEngine,
        cache: FloatingCache,
    }

    impl CachedLayouts {
        fn new() -> Self {
            let calls = Arc::new(AtomicUsize::new(0));
            let engine = CountingEngine {
                calls: calls.clone(),
            };
            Self {
                calls,
                engine,
                cache: HashMap::new(),
            }
        }

        /// Lays `content` out under the child `sizes`, and returns how many layout passes ran
        /// so far.
        fn lay_out(&mut self, content: &StyledNode, sizes: Option<&ChildSizesMap>) -> usize {
            let monitor_id = MonitorId::new("DP-1");
            let mut canvas_factory = MockCanvasFactory;
            layout_floating_content_cached(
                FloatingEngine::new(&mut self.engine, &mut self.cache),
                &mut pass(&monitor_id, &mut canvas_factory, sizes),
                content,
            );
            self.calls.load(Ordering::SeqCst)
        }
    }

    #[test]
    fn test_cached_layout_skips_recompute_for_unchanged_content() {
        let mut layouts = CachedLayouts::new();
        let content = text_node("unchanged");

        assert_eq!(
            layouts.lay_out(&content, None),
            1,
            "first call must run a real layout pass"
        );

        // Something else on the render changed (simulated by re-invoking
        // with byte-identical content) — the floating content itself did
        // not, so this must be a cache hit, not a second taffy pass.
        assert_eq!(
            layouts.lay_out(&content, None),
            1,
            "unchanged content must not re-run layout"
        );
    }

    /// Issue #9 regression: identical content whose embedded child has since
    /// reported (or changed) a size must be laid out again — the host box is
    /// measured from that report. A stale hit left the host unsettled
    /// forever, so the gate never opened and the popup never appeared.
    #[test]
    fn test_cached_layout_recomputes_when_child_sizes_change() {
        use crate::shared::primitives::{ChildReport, ModuleKey, ModuleName};

        let mut layouts = CachedLayouts::new();
        let content = text_node("popup with an embedded calendar");
        assert_eq!(layouts.lay_out(&content, None), 1);

        let mut sizes = ChildSizesMap::new();
        sizes.insert_report(
            ModuleKey::from_name(ModuleName::new("calendar")),
            ChildReport::new(Size::new(240, 215), Some(280)),
        );
        layouts.lay_out(&content, Some(&sizes));
        assert_eq!(
            layouts.lay_out(&content, Some(&sizes)),
            2,
            "a new child report must re-run layout once, then hit the cache"
        );
    }

    #[test]
    fn test_cached_layout_recomputes_for_changed_content() {
        let mut layouts = CachedLayouts::new();
        assert_eq!(layouts.lay_out(&text_node("first"), None), 1);
        assert_eq!(
            layouts.lay_out(&text_node("second"), None),
            2,
            "changed content must re-run layout"
        );
    }

    /// A layout engine that always fails.
    struct FailingEngine;

    impl LayoutEnginePort for FailingEngine {
        fn calculate_layout(
            &mut self,
            _root: LayoutRoot,
            _measurer: &mut dyn TextMeasurer,
        ) -> Result<RenderNode, LayoutError> {
            Err(LayoutError::EngineError("no layout".to_string()))
        }
    }

    /// A failed layout lays out nothing, caches nothing and is logged.
    #[test]
    fn test_failed_layout_is_logged_and_not_cached() {
        let mut canvas_factory = MockCanvasFactory;
        let mut cache = HashMap::new();
        let monitor_id = MonitorId::new("DP-1");
        let logs = crate::features::module_runtime::test_support::capture_logs(|| {
            let engine = FloatingEngine {
                engine: &mut FailingEngine,
                cache: &mut cache,
            };
            let mut pass = pass(&monitor_id, &mut canvas_factory, None);
            assert_eq!(
                layout_floating_content_cached(engine, &mut pass, &text_node("tip")),
                None
            );
        });
        assert!(cache.is_empty());
        assert!(logs.contains(
            "ERROR cranky::features::module_runtime::domain::render_pipeline::floating_phase::\
             content: Floating content layout calculation failed err=EngineError(\"no layout\")"
        ));
    }
}
