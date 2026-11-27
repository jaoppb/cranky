#[cfg(test)]
mod tests {
    use crate::features::module_runtime::domain::render_pipeline::pipeline::RenderPipeline;
    use crate::features::module_runtime::test_support::{
        MockLayoutEngine, MockVdomDiff, PipelineFixture, TestModulePort, capture_logs,
        process_monitor,
    };
    use crate::features::vdom::domain::{ElementSelectors, Interactions, VNode};
    use crate::shared::primitives::geometry::{Position, Rect, Size};
    use crate::shared::primitives::{ChildReport, ChildSizesMap, ModuleKey, ModuleName, MonitorId};

    fn rect_port() -> TestModulePort {
        TestModulePort::new(VNode::new_rect(
            ElementSelectors::default(),
            Interactions::default(),
        ))
    }

    /// Only the sizes embedded children reported changed: the diff says so and
    /// nothing else, and the layout phase lays the tree out again and records
    /// them, so the same sizes on the next render are no change at all.
    #[test]
    fn test_child_sizes_alone_trigger_one_relayout() {
        let mut pipeline = RenderPipeline::new();
        let mon = MonitorId::new("DP-1");
        let port = rect_port();
        let mut fixture = PipelineFixture::<MockLayoutEngine>::default();
        let mut sizes = ChildSizesMap::new();
        let report = ChildReport::new(Size::new(5, 5), None);
        sizes.insert_report(ModuleKey::from_name(ModuleName::new("calendar")), report);

        let mut ctx = fixture.ctx(&mon);
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
        let again = pipeline.diff(&port, ctx.diff_input(&MockVdomDiff));
        assert!(again.is_none());
    }

    /// New bounds alone are reported as a bounds change, not as a dirty tree.
    #[test]
    fn test_bounds_alone_are_a_bounds_change() {
        let mut pipeline = RenderPipeline::new();
        let mon = MonitorId::new("DP-1");
        let port = rect_port();
        let mut fixture = PipelineFixture::<MockLayoutEngine>::default();
        assert!(process_monitor(&mut pipeline, &port, fixture.ctx(&mon)).is_some());

        let mut ctx = fixture.ctx(&mon);
        ctx.current_bounds = Some(Rect::new(Position::new(0, 0), Size::new(10, 10)));
        let diff = pipeline
            .diff(&port, ctx.diff_input(&MockVdomDiff))
            .expect("new bounds are a change");
        assert!(!diff.vdom_dirty());
        assert!(diff.bounds_changed());
        assert!(!diff.child_sizes_changed());
    }

    /// A diff in which only the bounds changed lays the tree out again rather than reusing it.
    #[test]
    fn test_bounds_alone_trigger_a_relayout() {
        let mut pipeline = RenderPipeline::new();
        let mon = MonitorId::new("DP-1");
        let port = rect_port();
        let mut fixture = PipelineFixture::<MockLayoutEngine>::default();
        assert!(process_monitor(&mut pipeline, &port, fixture.ctx(&mon)).is_some());

        let mut ctx = fixture.ctx(&mon);
        ctx.current_bounds = Some(Rect::new(Position::new(0, 0), Size::new(10, 10)));
        let diff = pipeline
            .diff(&port, ctx.diff_input(&MockVdomDiff))
            .expect("new bounds are a change");
        let logs = capture_logs(|| {
            assert!(pipeline.layout(diff, &mut ctx).is_some());
        });
        assert!(logs.contains(
            "Updating layout for module monitor=DP-1 vdom_dirty=false bounds_changed=true \
             child_sizes_changed=false"
        ));
    }
}
