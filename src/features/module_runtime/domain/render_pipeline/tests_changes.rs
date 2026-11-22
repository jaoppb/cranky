#[cfg(test)]
mod tests {
    use crate::features::module_runtime::domain::render_pipeline::context::LayoutContext;
    use crate::features::module_runtime::domain::render_pipeline::pipeline::RenderPipeline;
    use crate::features::module_runtime::test_support::{
        MockCanvasFactory, MockLayoutEngine, MockStyleResolver, MockVdomDiff, PipelineFixture,
        TestModulePort, process_monitor,
    };
    use crate::features::vdom::domain::{ElementSelectors, Interactions, VNode};
    use crate::shared::primitives::MonitorId;
    use crate::shared::primitives::geometry::{Position, Rect, Scale, Size};
    use std::collections::HashMap;

    /// Only the sizes embedded children reported changed: the diff says so and
    /// nothing else, and the layout phase lays the tree out again and records
    /// them, so the same sizes on the next render are no change at all.
    #[test]
    fn test_child_sizes_alone_trigger_one_relayout() {
        use crate::shared::primitives::{ChildReport, ChildSizesMap, ModuleKey, ModuleName};

        let mut pipeline = RenderPipeline::new();
        let mon = MonitorId::new("DP-1");
        let port = TestModulePort::new(VNode::new_rect(
            ElementSelectors::default(),
            Interactions::default(),
        ));
        let resolver = MockStyleResolver;
        let mut factory = MockCanvasFactory;
        let mut engine = MockLayoutEngine;
        let mut popup_engine = MockLayoutEngine;
        let mut panel_engine = MockLayoutEngine;
        let mut tooltip_engine = MockLayoutEngine;
        let child_errors = HashMap::new();
        let mut sizes = ChildSizesMap::new();
        sizes.insert_report(
            ModuleKey::from_name(ModuleName::new("calendar")),
            ChildReport::new(Size::new(5, 5), None),
        );

        let mut ctx = LayoutContext {
            monitor_id: &mon,
            scale: Scale::new(1.0),
            style_resolver: &resolver,
            child_errors: &child_errors,
            current_bounds: None,
            current_constraint: crate::shared::primitives::SizeConstraint::none(),
            current_child_sizes: None,
            interaction_context: None,
            canvas_factory: &mut factory,
            layout_engine: &mut engine,
            popup_layout_engine: &mut popup_engine,
            panel_layout_engine: &mut panel_engine,
            tooltip_layout_engine: &mut tooltip_engine,
        };
        let first = pipeline.diff(&port, ctx.diff_input(&MockVdomDiff));
        assert!(pipeline.layout(first.unwrap(), &mut ctx).is_some());

        ctx.current_child_sizes = Some(&sizes);
        let diff = pipeline
            .diff(&port, ctx.diff_input(&MockVdomDiff))
            .expect("new child sizes are a change");
        assert!(!diff.vdom_dirty());
        assert!(!diff.bounds_changed());
        assert!(diff.child_sizes_changed());

        assert!(pipeline.layout(diff, &mut ctx).is_some());
        assert!(
            pipeline
                .diff(&port, ctx.diff_input(&MockVdomDiff))
                .is_none()
        );
    }

    /// New bounds alone are reported as a bounds change, not as a dirty tree.
    #[test]
    fn test_bounds_alone_are_a_bounds_change() {
        let mut pipeline = RenderPipeline::new();
        let mon = MonitorId::new("DP-1");
        let port = TestModulePort::new(VNode::new_rect(
            ElementSelectors::default(),
            Interactions::default(),
        ));
        let resolver = MockStyleResolver;
        let mut factory = MockCanvasFactory;
        let mut engine = MockLayoutEngine;
        let mut popup_engine = MockLayoutEngine;
        let mut panel_engine = MockLayoutEngine;
        let mut tooltip_engine = MockLayoutEngine;
        let child_errors = HashMap::new();
        let ctx = LayoutContext {
            monitor_id: &mon,
            scale: Scale::new(1.0),
            style_resolver: &resolver,
            child_errors: &child_errors,
            current_bounds: None,
            current_constraint: crate::shared::primitives::SizeConstraint::none(),
            current_child_sizes: None,
            interaction_context: None,
            canvas_factory: &mut factory,
            layout_engine: &mut engine,
            popup_layout_engine: &mut popup_engine,
            panel_layout_engine: &mut panel_engine,
            tooltip_layout_engine: &mut tooltip_engine,
        };
        assert!(process_monitor(&mut pipeline, &port, ctx).is_some());

        let mut fixture = PipelineFixture::<MockLayoutEngine>::default();
        let mut ctx = fixture.ctx(&mon);
        ctx.current_bounds = Some(Rect::new(Position::new(0, 0), Size::new(10, 10)));
        let diff = pipeline
            .diff(&port, ctx.diff_input(&MockVdomDiff))
            .expect("new bounds are a change");
        assert!(!diff.vdom_dirty());
        assert!(diff.bounds_changed());
        assert!(!diff.child_sizes_changed());
    }
}
