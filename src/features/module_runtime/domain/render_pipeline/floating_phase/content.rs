use super::super::measurer::ModuleSizeMeasurer;
use crate::features::layout_engine::domain::{RenderNode, StyledNode};
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::geometry::{Position, Scale};
use crate::shared::primitives::{ChildSizesMap, MonitorId};
use crate::shared::rendering::ports::canvas::CanvasFactory;
use std::collections::HashMap;

/// Last laid-out floating content per monitor, with its child-size snapshot.
///
/// An embedded module's host box is measured from what the child reported,
/// so the same content under different reports is a different layout.
pub type FloatingCache = HashMap<MonitorId, (StyledNode, Option<ChildSizesMap>, RenderNode)>;

/// Fallback font used when a floating node's own CSS specifies none —
/// matches `paint_pipeline`'s fallback so a popup's fonts never disagree
/// with what the bar tree would use for the same unstyled node.
const fn floating_font_fallback() -> (FontFamily, FontSize) {
    (FontFamily::new(String::new()), FontSize::new(14.0))
}

/// Lays out one piece of floating content (a popup's, panel's or tooltip's)
/// with the pipeline's own measurer, scale and font fallback — shared by all
/// three so hit-testing and painting always agree, whichever kind it is.
pub(super) fn layout_floating_content<F: CanvasFactory, E: LayoutEnginePort>(
    layout_engine: &mut E,
    canvas_factory: &mut F,
    scale: Scale,
    current_child_sizes: Option<&ChildSizesMap>,
    content: &StyledNode,
) -> Option<RenderNode> {
    let (font_family, font_size) = floating_font_fallback();
    let measurer_inner = canvas_factory.create_text_measurer(scale, font_family, font_size);
    let mut measurer = ModuleSizeMeasurer::new(measurer_inner, current_child_sizes);
    match layout_engine.calculate_layout(content.clone(), &mut measurer, Position::new(0, 0)) {
        Ok(node) => Some(node),
        Err(e) => {
            tracing::error!(err = ?e, "Floating content layout calculation failed");
            None
        }
    }
}

/// `layout_floating_content`, but skips the taffy pass entirely when
/// `content` compares equal to whatever was last laid out for this kind on
/// this monitor — a popup/panel/tooltip left open and unchanged otherwise
/// pays a full layout + clone on every render where anything else (a
/// sibling module's size, say) is what's actually dirty.
#[allow(clippy::too_many_arguments)]
pub(super) fn layout_floating_content_cached<F: CanvasFactory, E: LayoutEnginePort>(
    cache: &mut FloatingCache,
    monitor_id: &MonitorId,
    layout_engine: &mut E,
    canvas_factory: &mut F,
    scale: Scale,
    current_child_sizes: Option<&ChildSizesMap>,
    content: &StyledNode,
) -> Option<RenderNode> {
    if let Some((cached_content, cached_sizes, cached_node)) = cache.get(monitor_id)
        && cached_content == content
        && cached_sizes.as_ref() == current_child_sizes
    {
        return Some(cached_node.clone());
    }
    let node = layout_floating_content(layout_engine, canvas_factory, scale, current_child_sizes, content)?;
    cache.insert(
        monitor_id.clone(),
        (content.clone(), current_child_sizes.cloned(), node.clone()),
    );
    Some(node)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::layout_engine::domain::{LayoutError, NodePath, SurfaceSpace, TextMeasurer};
    use crate::features::module_runtime::test_support::MockCanvasFactory;
    use crate::features::styling::domain::ComputedStyle;
    use crate::shared::primitives::SizeConstraint;
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
        fn calculate_layout_with_constraints(
            &mut self,
            _node: StyledNode,
            _measurer: &mut dyn TextMeasurer,
            _start_pos: Position,
            _constraint: SizeConstraint,
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

    #[test]
    fn test_cached_layout_skips_recompute_for_unchanged_content() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut engine = CountingEngine { calls: calls.clone() };
        let mut canvas_factory = MockCanvasFactory;
        let mut cache = HashMap::new();
        let monitor_id = MonitorId::new("DP-1");
        let content = text_node("unchanged");

        layout_floating_content_cached(
            &mut cache, &monitor_id, &mut engine, &mut canvas_factory, Scale::new(1.0), None, &content,
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1, "first call must run a real layout pass");

        // Something else on the render changed (simulated by re-invoking
        // with byte-identical content) — the floating content itself did
        // not, so this must be a cache hit, not a second taffy pass.
        layout_floating_content_cached(
            &mut cache, &monitor_id, &mut engine, &mut canvas_factory, Scale::new(1.0), None, &content,
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1, "unchanged content must not re-run layout");
    }

    /// Issue #9 regression: identical content whose embedded child has since
    /// reported (or changed) a size must be laid out again — the host box is
    /// measured from that report. A stale hit left the host unsettled
    /// forever, so the gate never opened and the popup never appeared.
    #[test]
    fn test_cached_layout_recomputes_when_child_sizes_change() {
        use crate::shared::primitives::{ChildReport, ChildSizesMap, ModuleKey, ModuleName};

        let calls = Arc::new(AtomicUsize::new(0));
        let mut engine = CountingEngine { calls: calls.clone() };
        let mut canvas_factory = MockCanvasFactory;
        let mut cache = HashMap::new();
        let monitor_id = MonitorId::new("DP-1");
        let content = text_node("popup with an embedded calendar");

        layout_floating_content_cached(
            &mut cache, &monitor_id, &mut engine, &mut canvas_factory, Scale::new(1.0), None, &content,
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let mut sizes = ChildSizesMap::new();
        sizes.insert_report(
            ModuleKey::from_name(ModuleName::new("calendar")),
            ChildReport::new(Size::new(240, 215), Some(280)),
        );
        for _ in 0..2 {
            layout_floating_content_cached(
                &mut cache, &monitor_id, &mut engine, &mut canvas_factory, Scale::new(1.0), Some(&sizes), &content,
            );
        }
        assert_eq!(
            calls.load(Ordering::SeqCst),
            2,
            "a new child report must re-run layout once, then hit the cache"
        );
    }

    #[test]
    fn test_cached_layout_recomputes_for_changed_content() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut engine = CountingEngine { calls: calls.clone() };
        let mut canvas_factory = MockCanvasFactory;
        let mut cache = HashMap::new();
        let monitor_id = MonitorId::new("DP-1");

        layout_floating_content_cached(
            &mut cache, &monitor_id, &mut engine, &mut canvas_factory, Scale::new(1.0), None, &text_node("first"),
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        layout_floating_content_cached(
            &mut cache, &monitor_id, &mut engine, &mut canvas_factory, Scale::new(1.0), None, &text_node("second"),
        );
        assert_eq!(calls.load(Ordering::SeqCst), 2, "changed content must re-run layout");
    }
}
