use crate::features::layout_engine::domain::{RenderNode, TextMeasurer};
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::module_runtime::ports::AnyModulePort;
use crate::features::styling::ports::StyleResolverPort;
use crate::features::vdom::domain::VNode;
use crate::features::vdom::ports::VdomDiffPort;
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::geometry::{Position, Rect, Scale, Size};
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::primitives::{ChildModuleLayout, ChildSizesMap, ModuleKey, MonitorId};
use crate::shared::rendering::ports::canvas::CanvasFactory;
use std::collections::HashMap;

/// Value Object representing a module size transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizeChange {
    old: Size,
    new: Size,
}

impl SizeChange {
    #[must_use]
    pub const fn new(old: Size, new: Size) -> Self {
        Self { old, new }
    }

    #[must_use]
    pub const fn old(&self) -> Size {
        self.old
    }

    #[must_use]
    pub const fn new_size(&self) -> Size {
        self.new
    }
}

#[derive(Debug, Clone)]
pub struct RenderOutcome {
    size_change: Option<SizeChange>,
    child_layouts: Vec<ChildModuleLayout>,
    render_tree: RenderNode,
    buffer: Option<(RenderBuffer, Position)>,
}

impl RenderOutcome {
    #[must_use]
    pub const fn new(
        size_change: Option<SizeChange>,
        child_layouts: Vec<ChildModuleLayout>,
        render_tree: RenderNode,
        buffer: Option<(RenderBuffer, Position)>,
    ) -> Self {
        Self {
            size_change,
            child_layouts,
            render_tree,
            buffer,
        }
    }

    #[must_use]
    pub const fn size_change(&self) -> Option<&SizeChange> {
        self.size_change.as_ref()
    }

    #[must_use]
    pub fn child_layouts(&self) -> &[ChildModuleLayout] {
        &self.child_layouts
    }

    #[must_use]
    pub const fn render_tree(&self) -> &RenderNode {
        &self.render_tree
    }

    #[must_use]
    pub const fn buffer(&self) -> Option<&(RenderBuffer, Position)> {
        self.buffer.as_ref()
    }

    #[must_use]
    pub fn into_buffer(self) -> Option<(RenderBuffer, Position)> {
        self.buffer
    }
}

pub struct ModuleSizeMeasurer<'a, M: TextMeasurer> {
    inner: M,
    child_sizes: Option<&'a ChildSizesMap>,
}

impl<'a, M: TextMeasurer> ModuleSizeMeasurer<'a, M> {
    #[must_use]
    pub const fn new(inner: M, child_sizes: Option<&'a ChildSizesMap>) -> Self {
        Self { inner, child_sizes }
    }
}

impl<M: TextMeasurer> TextMeasurer for ModuleSizeMeasurer<'_, M> {
    fn measure(&mut self, text: &str, font: Option<&FontFamily>, size: Option<FontSize>) -> Size {
        self.inner.measure(text, font, size)
    }

    fn measure_module(&self, key: &ModuleKey) -> Option<Size> {
        let size = self.child_sizes.and_then(|sizes| {
            sizes
                .get_by_name_or_key(key.name(), key.instance_id())
                .copied()
        });
        tracing::trace!(
            ?key,
            ?size,
            has_child_sizes = self.child_sizes.is_some(),
            "measure_module called"
        );
        size
    }
}

pub struct LayoutContext<'a, F: CanvasFactory> {
    pub style_resolver: &'a dyn StyleResolverPort,
    pub current_bounds: Option<Rect>,
    pub current_child_sizes: Option<&'a ChildSizesMap>,
    pub canvas_factory: &'a mut F,
    pub layout_engine: &'a mut dyn LayoutEnginePort,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PipelineDiff {
    new_vdom: VNode,
    vdom_dirty: bool,
    bounds_changed: bool,
    child_sizes_changed: bool,
}

impl PipelineDiff {
    #[must_use]
    pub const fn new(
        new_vdom: VNode,
        vdom_dirty: bool,
        bounds_changed: bool,
        child_sizes_changed: bool,
    ) -> Self {
        Self {
            new_vdom,
            vdom_dirty,
            bounds_changed,
            child_sizes_changed,
        }
    }

    #[must_use]
    pub const fn new_vdom(&self) -> &VNode {
        &self.new_vdom
    }

    #[must_use]
    pub const fn vdom_dirty(&self) -> bool {
        self.vdom_dirty
    }

    #[must_use]
    pub const fn bounds_changed(&self) -> bool {
        self.bounds_changed
    }

    #[must_use]
    pub const fn child_sizes_changed(&self) -> bool {
        self.child_sizes_changed
    }
}

#[derive(Debug, Default)]
pub struct RenderPipeline {
    sizes: HashMap<MonitorId, Size>,
    rendered_bounds: HashMap<MonitorId, Rect>,
    render_trees: HashMap<MonitorId, RenderNode>,
    vdom_trees: HashMap<MonitorId, VNode>,
    last_child_sizes: HashMap<MonitorId, Option<ChildSizesMap>>,
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

    #[must_use]
    pub const fn rendered_bounds(&self) -> &HashMap<MonitorId, Rect> {
        &self.rendered_bounds
    }

    #[must_use]
    pub const fn render_trees(&self) -> &HashMap<MonitorId, RenderNode> {
        &self.render_trees
    }

    #[must_use]
    pub const fn vdom_trees(&self) -> &HashMap<MonitorId, VNode> {
        &self.vdom_trees
    }

    #[must_use]
    pub const fn last_child_sizes(&self) -> &HashMap<MonitorId, Option<ChildSizesMap>> {
        &self.last_child_sizes
    }

    #[must_use]
    pub fn diff(
        &self,
        monitor_id: &MonitorId,
        port: &dyn AnyModulePort,
        vdom_diff: &dyn VdomDiffPort,
        current_bounds: Option<Rect>,
        current_child_sizes: Option<&ChildSizesMap>,
    ) -> Option<PipelineDiff> {
        let new_vdom = port.render(monitor_id);
        let diff_result = vdom_diff.diff(self.vdom_trees.get(monitor_id), &new_vdom);

        let current_child_sizes_owned = current_child_sizes.cloned();
        let bounds_changed = current_bounds != self.rendered_bounds.get(monitor_id).copied();
        let child_sizes_changed =
            self.last_child_sizes.get(monitor_id) != Some(&current_child_sizes_owned);

        if diff_result.is_unchanged()
            && !bounds_changed
            && !child_sizes_changed
            && self.render_trees.contains_key(monitor_id)
        {
            tracing::trace!(
                monitor = %monitor_id,
                "VDOM, bounds, and child sizes unchanged; skipping style resolution, layout, and canvas render"
            );
            return None;
        }

        let vdom_dirty = !diff_result.is_unchanged() || !self.render_trees.contains_key(monitor_id);

        Some(PipelineDiff::new(
            new_vdom,
            vdom_dirty,
            bounds_changed,
            child_sizes_changed,
        ))
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn layout<F: CanvasFactory>(
        &mut self,
        monitor_id: &MonitorId,
        diff: PipelineDiff,
        ctx: &mut LayoutContext<'_, F>,
    ) -> Option<(RenderNode, Option<SizeChange>, Vec<ChildModuleLayout>)> {
        let current_child_sizes_owned = ctx.current_child_sizes.cloned();
        let mut size_change = None;
        let mut child_layouts = Vec::new();

        let render_node = if diff.vdom_dirty || diff.bounds_changed || diff.child_sizes_changed {
            self.last_child_sizes
                .insert(monitor_id.clone(), current_child_sizes_owned);
            tracing::trace!(
                monitor = %monitor_id,
                vdom_dirty = diff.vdom_dirty,
                bounds_changed = diff.bounds_changed,
                child_sizes_changed = diff.child_sizes_changed,
                "Updating layout for module"
            );

            self.vdom_trees
                .insert(monitor_id.clone(), diff.new_vdom.clone());

            tracing::trace!(monitor = %monitor_id, "Resolving styles for module VNode");
            let styled_node = diff.new_vdom.resolve_styles(ctx.style_resolver, None);

            let default_font_family = FontFamily::new(String::new());
            let default_font_size = FontSize::new(14.0);

            let available_size = ctx
                .current_bounds
                .filter(|b| b.width() > 0 && b.height() > 0)
                .map(|b| *b.size());

            let measurer_inner = ctx.canvas_factory.create_text_measurer(
                Scale::new(1.0),
                default_font_family,
                default_font_size,
            );
            let mut measurer = ModuleSizeMeasurer::new(measurer_inner, ctx.current_child_sizes);

            let render_node_res = ctx.layout_engine.calculate_layout_with_constraints(
                styled_node,
                &mut measurer,
                Position::new(0, 0),
                available_size,
            );

            let render_node = match render_node_res {
                Ok(node) => node,
                Err(e) => {
                    tracing::error!(monitor = %monitor_id, err = ?e, "Module layout calculation failed");
                    return None;
                }
            };

            child_layouts = render_node.collect_module_layouts();

            self.render_trees
                .insert(monitor_id.clone(), render_node.clone());

            let size = *render_node.rect().size();
            let old_size = self
                .sizes
                .get(monitor_id)
                .copied()
                .unwrap_or(Size::new(0, 0));

            if size != old_size {
                self.sizes.insert(monitor_id.clone(), size);
                tracing::trace!(
                    monitor = %monitor_id,
                    ?size,
                    ?old_size,
                    "Module size changed"
                );
                size_change = Some(SizeChange::new(old_size, size));
            }

            render_node
        } else {
            self.render_trees.get(monitor_id)?.clone()
        };

        Some((render_node, size_change, child_layouts))
    }

    pub fn paint<F: CanvasFactory>(
        &mut self,
        monitor_id: &MonitorId,
        render_node: &RenderNode,
        current_bounds: Option<Rect>,
        canvas_factory: &mut F,
    ) -> Option<(RenderBuffer, Position)> {
        let bounds = current_bounds.filter(|b| b.width() > 0 && b.height() > 0)?;

        let default_font_family = FontFamily::new(String::new());
        let default_font_size = FontSize::new(14.0);

        let width = usize::try_from(bounds.width()).unwrap_or(0);
        let height = usize::try_from(bounds.height()).unwrap_or(0);
        let len = width.saturating_mul(height).saturating_mul(4);
        let mut data = vec![0u8; len];
        {
            let mut canvas = canvas_factory.create_canvas(
                &mut data,
                *bounds.size(),
                Scale::new(1.0),
                default_font_family,
                default_font_size,
            );
            render_node.render_to_canvas(&mut canvas);
        }

        let render_buf = RenderBuffer::new(data, *bounds.size());
        let position = Position::new(bounds.x(), bounds.y());
        self.rendered_bounds.insert(monitor_id.clone(), bounds);
        Some((render_buf, position))
    }

    pub fn process_monitor<F: CanvasFactory>(
        &mut self,
        monitor_id: &MonitorId,
        port: &dyn AnyModulePort,
        vdom_diff: &dyn VdomDiffPort,
        mut ctx: LayoutContext<'_, F>,
    ) -> Option<RenderOutcome> {
        let diff = self.diff(
            monitor_id,
            port,
            vdom_diff,
            ctx.current_bounds,
            ctx.current_child_sizes,
        )?;

        let current_bounds = ctx.current_bounds;
        let (render_node, size_change, child_layouts) = self.layout(monitor_id, diff, &mut ctx)?;

        let buffer = self.paint(monitor_id, &render_node, current_bounds, ctx.canvas_factory);

        Some(RenderOutcome::new(
            size_change,
            child_layouts,
            render_node,
            buffer,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::module_runtime::test_support::{MockCanvasFactory, TestModulePort};
    use crate::features::styling::adapters::fs_loader::CompositeStyleResolver;
    use crate::features::vdom::adapters::DefaultVdomDiffAdapter;
    use crate::features::vdom::domain::VNode;

    #[test]
    fn test_pipeline_diff_and_layout_phases() {
        let mut pipeline = RenderPipeline::new();
        let mon = MonitorId::new("DP-1");
        let port = TestModulePort::new(VNode::new_rect(None, None, None, None, None));
        let diff_adapter = DefaultVdomDiffAdapter::new();
        let resolver = CompositeStyleResolver::new(vec![]);
        let mut factory = MockCanvasFactory;
        let mut engine = crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter::new();

        // 1. Diff Phase
        let diff = pipeline.diff(&mon, &port, &diff_adapter, None, None);
        assert!(diff.is_some());
        let diff = diff.unwrap();
        assert!(diff.vdom_dirty());

        // 2. Layout Phase
        let mut ctx = LayoutContext {
            style_resolver: &resolver,
            current_bounds: None,
            current_child_sizes: None,
            canvas_factory: &mut factory,
            layout_engine: &mut engine,
        };
        let layout_res = pipeline.layout(&mon, diff, &mut ctx);
        assert!(layout_res.is_some());
        let (node, size_change, child_layouts) = layout_res.unwrap();
        assert_eq!(
            size_change,
            Some(SizeChange::new(Size::new(0, 0), Size::new(10, 10)))
        );
        assert!(child_layouts.is_empty());
        assert_eq!(*node.rect().size(), Size::new(10, 10));

        // 3. Paint Phase without bounds returns None
        let paint_res = pipeline.paint(&mon, &node, None, &mut factory);
        assert!(paint_res.is_none());

        // 4. Paint Phase with valid bounds returns buffer
        let bounds = Rect::new(Position::new(0, 0), Size::new(10, 10));
        let paint_res = pipeline.paint(&mon, &node, Some(bounds), &mut factory);
        assert!(paint_res.is_some());
        let (_buf, pos) = paint_res.unwrap();
        assert_eq!(pos, Position::new(0, 0));
    }

    #[test]
    fn test_process_monitor_initial_render_and_size_change() {
        let mut pipeline = RenderPipeline::new();
        let mon = MonitorId::new("DP-1");
        let port = TestModulePort::new(VNode::new_rect(None, None, None, None, None));
        let diff = DefaultVdomDiffAdapter::new();
        let resolver = CompositeStyleResolver::new(vec![]);
        let mut factory = MockCanvasFactory;
        let mut engine = crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter::new();

        let ctx = LayoutContext {
            style_resolver: &resolver,
            current_bounds: None,
            current_child_sizes: None,
            canvas_factory: &mut factory,
            layout_engine: &mut engine,
        };
        let outcome = pipeline.process_monitor(&mon, &port, &diff, ctx);

        assert!(outcome.is_some());
        let outcome = outcome.unwrap();
        assert_eq!(
            outcome.size_change(),
            Some(&SizeChange::new(Size::new(0, 0), Size::new(10, 10)))
        );
        assert!(outcome.buffer().is_none());
        assert!(pipeline.render_trees().contains_key(&mon));
        assert!(pipeline.vdom_trees().contains_key(&mon));
    }

    #[test]
    fn test_process_monitor_unchanged_returns_none() {
        let mut pipeline = RenderPipeline::new();
        let mon = MonitorId::new("DP-1");
        let port = TestModulePort::new(VNode::new_rect(None, None, None, None, None));
        let diff = DefaultVdomDiffAdapter::new();
        let resolver = CompositeStyleResolver::new(vec![]);
        let mut factory = MockCanvasFactory;
        let mut engine = crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter::new();

        let ctx1 = LayoutContext {
            style_resolver: &resolver,
            current_bounds: None,
            current_child_sizes: None,
            canvas_factory: &mut factory,
            layout_engine: &mut engine,
        };
        let outcome1 = pipeline.process_monitor(&mon, &port, &diff, ctx1);
        assert!(outcome1.is_some());

        // Second run with no changes should return None (early exit)
        let ctx2 = LayoutContext {
            style_resolver: &resolver,
            current_bounds: None,
            current_child_sizes: None,
            canvas_factory: &mut factory,
            layout_engine: &mut engine,
        };
        let outcome2 = pipeline.process_monitor(&mon, &port, &diff, ctx2);
        assert!(outcome2.is_none());
    }
}
