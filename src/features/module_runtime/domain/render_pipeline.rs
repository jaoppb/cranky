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
    pub fn new(old: Size, new: Size) -> Self {
        Self { old, new }
    }

    pub fn old(&self) -> Size {
        self.old
    }

    pub fn new_size(&self) -> Size {
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
    pub fn new(
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

    pub fn size_change(&self) -> Option<&SizeChange> {
        self.size_change.as_ref()
    }

    pub fn child_layouts(&self) -> &[ChildModuleLayout] {
        &self.child_layouts
    }

    pub fn render_tree(&self) -> &RenderNode {
        &self.render_tree
    }

    pub fn buffer(&self) -> Option<&(RenderBuffer, Position)> {
        self.buffer.as_ref()
    }

    pub fn into_buffer(self) -> Option<(RenderBuffer, Position)> {
        self.buffer
    }
}

pub struct ModuleSizeMeasurer<'a, M: TextMeasurer> {
    inner: M,
    child_sizes: Option<&'a ChildSizesMap>,
}

impl<'a, M: TextMeasurer> ModuleSizeMeasurer<'a, M> {
    pub fn new(inner: M, child_sizes: Option<&'a ChildSizesMap>) -> Self {
        Self { inner, child_sizes }
    }
}

impl<'a, M: TextMeasurer> TextMeasurer for ModuleSizeMeasurer<'a, M> {
    fn measure(
        &mut self,
        text: &str,
        font: Option<&FontFamily>,
        size: Option<FontSize>,
    ) -> Size {
        self.inner.measure(text, font, size)
    }

    fn measure_module(&self, key: &ModuleKey) -> Option<Size> {
        let size = self
            .child_sizes
            .and_then(|sizes| sizes.get_by_name_or_key(key.name(), key.instance_id()).copied());
        tracing::trace!(?key, ?size, has_child_sizes = self.child_sizes.is_some(), "measure_module called");
        size
    }
}

pub struct ProcessMonitorContext<'a, F: CanvasFactory> {
    port: &'a dyn AnyModulePort,
    vdom_diff: &'a dyn VdomDiffPort,
    style_resolver: &'a dyn StyleResolverPort,
    current_bounds: Option<Rect>,
    current_child_sizes: Option<&'a ChildSizesMap>,
    canvas_factory: &'a mut F,
    layout_engine: &'a mut dyn LayoutEnginePort,
}

impl<'a, F: CanvasFactory> ProcessMonitorContext<'a, F> {
    pub fn new(
        port: &'a dyn AnyModulePort,
        vdom_diff: &'a dyn VdomDiffPort,
        style_resolver: &'a dyn StyleResolverPort,
        current_bounds: Option<Rect>,
        current_child_sizes: Option<&'a ChildSizesMap>,
        canvas_factory: &'a mut F,
        layout_engine: &'a mut dyn LayoutEnginePort,
    ) -> Self {
        Self {
            port,
            vdom_diff,
            style_resolver,
            current_bounds,
            current_child_sizes,
            canvas_factory,
            layout_engine,
        }
    }

    pub fn port(&self) -> &dyn AnyModulePort {
        self.port
    }

    pub fn vdom_diff(&self) -> &dyn VdomDiffPort {
        self.vdom_diff
    }

    pub fn style_resolver(&self) -> &dyn StyleResolverPort {
        self.style_resolver
    }

    pub fn current_bounds(&self) -> Option<Rect> {
        self.current_bounds
    }

    pub fn current_child_sizes(&self) -> Option<&ChildSizesMap> {
        self.current_child_sizes
    }

    pub fn canvas_factory_mut(&mut self) -> &mut F {
        self.canvas_factory
    }

    pub fn layout_engine_mut(&mut self) -> &mut dyn LayoutEnginePort {
        self.layout_engine
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sizes(&self) -> &HashMap<MonitorId, Size> {
        &self.sizes
    }

    pub fn rendered_bounds(&self) -> &HashMap<MonitorId, Rect> {
        &self.rendered_bounds
    }

    pub fn render_trees(&self) -> &HashMap<MonitorId, RenderNode> {
        &self.render_trees
    }

    pub fn vdom_trees(&self) -> &HashMap<MonitorId, VNode> {
        &self.vdom_trees
    }

    pub fn last_child_sizes(&self) -> &HashMap<MonitorId, Option<ChildSizesMap>> {
        &self.last_child_sizes
    }

    pub fn process_monitor<F: CanvasFactory>(
        &mut self,
        monitor_id: &MonitorId,
        ctx: ProcessMonitorContext<'_, F>,
    ) -> Option<RenderOutcome> {
        let ProcessMonitorContext {
            port,
            vdom_diff,
            style_resolver,
            current_bounds,
            current_child_sizes,
            canvas_factory,
            layout_engine,
        } = ctx;

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

        let vdom_dirty =
            !diff_result.is_unchanged() || !self.render_trees.contains_key(monitor_id);

        let mut size_change = None;
        let mut child_layouts = Vec::new();

        let render_node = if vdom_dirty || bounds_changed || child_sizes_changed {
            self.last_child_sizes
                .insert(monitor_id.clone(), current_child_sizes_owned);
            tracing::trace!(
                monitor = %monitor_id,
                vdom_dirty,
                bounds_changed,
                child_sizes_changed,
                "Updating layout for module"
            );

            self.vdom_trees
                .insert(monitor_id.clone(), new_vdom.clone());

            tracing::trace!(monitor = %monitor_id, "Resolving styles for module VNode");
            let styled_node = new_vdom.resolve_styles(style_resolver, None);

            let default_font_family = FontFamily::new("".to_string());
            let default_font_size = FontSize::new(14.0);

            let available_size = current_bounds
                .filter(|b| b.width() > 0 && b.height() > 0)
                .map(|b| *b.size());

            let measurer_inner = canvas_factory.create_text_measurer(
                Scale::new(1.0),
                default_font_family,
                default_font_size,
            );
            let mut measurer = ModuleSizeMeasurer::new(measurer_inner, current_child_sizes);

            let render_node_res = layout_engine.calculate_layout_with_constraints(
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
            self.render_trees.get(monitor_id).unwrap().clone()
        };

        let mut buffer = None;

        if let Some(bounds) = current_bounds
            && bounds.width() > 0
            && bounds.height() > 0
        {
            let default_font_family = FontFamily::new("".to_string());
            let default_font_size = FontSize::new(14.0);

            let w = bounds.width();
            let h = bounds.height();
            let mut data = vec![0u8; (w * h * 4) as usize];
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
            buffer = Some((render_buf, position));
        }

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
    use crate::features::module_runtime::ports::ModuleInitError;
    use crate::features::vdom::adapters::DefaultVdomDiffAdapter;
    use crate::features::vdom::domain::VNode;
    use crate::features::styling::adapters::fs_loader::CompositeStyleResolver;
    use crate::shared::config::domain::{Config, ModuleConfig};
    use crate::shared::events::signals::{SignalHub, SignalKind};
    use crate::shared::primitives::FunctionName;

    struct TestModulePort {
        node: VNode,
    }

    impl AnyModulePort for TestModulePort {
        fn init(&mut self, _config: &ModuleConfig, _full: &Config) -> Result<(), ModuleInitError> {
            Ok(())
        }
        fn subscriptions(&self) -> &[SignalKind] {
            &[]
        }
        fn styles(&self) -> &[crate::features::styling::domain::StyleSheetName] {
            &[]
        }
        fn refresh(&mut self, _hub: &SignalHub, _signals: &[SignalKind]) {}
        fn render(&self, _monitor: &MonitorId) -> VNode {
            self.node.clone()
        }
        fn call_function(&mut self, _name: &FunctionName) -> Result<(), ModuleInitError> {
            Ok(())
        }
    }

    struct MockCanvasFactory;

    impl CanvasFactory for MockCanvasFactory {
        fn create_canvas<'a>(
            &'a mut self,
            _data: &'a mut [u8],
            _size: Size,
            _scale: Scale,
            _font_family: FontFamily,
            _font_size: FontSize,
        ) -> impl crate::shared::rendering::ports::canvas::Canvas + 'a {
            MockCanvas
        }

        fn create_text_measurer<'a>(
            &'a mut self,
            _scale: Scale,
            _font_family: FontFamily,
            _font_size: FontSize,
        ) -> impl TextMeasurer + 'a {
            MockMeasurer
        }
    }

    struct MockCanvas;
    impl crate::shared::rendering::ports::canvas::Canvas for MockCanvas {
        fn draw_rect(
            &mut self,
            _x: crate::shared::primitives::geometry::LogicalPx,
            _y: crate::shared::primitives::geometry::LogicalPx,
            _w: crate::shared::primitives::geometry::LogicalPx,
            _h: crate::shared::primitives::geometry::LogicalPx,
            _color: crate::shared::primitives::color::DrawingColor,
            _radius: crate::shared::primitives::geometry::LogicalPx,
        ) {}
        fn draw_border(
            &mut self,
            _pos: Position,
            _size: Size,
            _color: crate::shared::primitives::color::DrawingColor,
            _radius: crate::shared::primitives::geometry::LogicalPx,
            _border_size: crate::shared::primitives::geometry::LogicalPx,
        ) {}
        fn draw_text(
            &mut self,
            _text: &str,
            _font_family: Option<&FontFamily>,
            _font_size: Option<FontSize>,
            _color: crate::shared::primitives::color::DrawingColor,
            _pos: Position,
        ) {}
        fn draw_image(
            &mut self,
            _image_data: &[u8],
            _pixel_size: Size,
            _logical_size: Size,
            _pos: Position,
        ) {}
    }

    struct MockMeasurer;
    impl TextMeasurer for MockMeasurer {
        fn measure(&mut self, _text: &str, _font: Option<&FontFamily>, _size: Option<FontSize>) -> Size {
            Size::new(10, 10)
        }
    }

    #[test]
    fn test_process_monitor_initial_render_and_size_change() {
        let mut pipeline = RenderPipeline::new();
        let mon = MonitorId::new("DP-1");
        let port = TestModulePort {
            node: VNode::new_rect(None, None, None, None, None),
        };
        let diff = DefaultVdomDiffAdapter::new();
        let resolver = CompositeStyleResolver::new(vec![]);
        let mut factory = MockCanvasFactory;
        let mut engine = crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter::new();

        let ctx = ProcessMonitorContext::new(
            &port,
            &diff,
            &resolver,
            None,
            None,
            &mut factory,
            &mut engine,
        );
        let outcome = pipeline.process_monitor(&mon, ctx);

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
        let port = TestModulePort {
            node: VNode::new_rect(None, None, None, None, None),
        };
        let diff = DefaultVdomDiffAdapter::new();
        let resolver = CompositeStyleResolver::new(vec![]);
        let mut factory = MockCanvasFactory;
        let mut engine = crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter::new();

        let ctx1 = ProcessMonitorContext::new(
            &port,
            &diff,
            &resolver,
            None,
            None,
            &mut factory,
            &mut engine,
        );
        let outcome1 = pipeline.process_monitor(&mon, ctx1);
        assert!(outcome1.is_some());

        // Second run with no changes should return None (early exit)
        let ctx2 = ProcessMonitorContext::new(
            &port,
            &diff,
            &resolver,
            None,
            None,
            &mut factory,
            &mut engine,
        );
        let outcome2 = pipeline.process_monitor(&mon, ctx2);
        assert!(outcome2.is_none());
    }
}
