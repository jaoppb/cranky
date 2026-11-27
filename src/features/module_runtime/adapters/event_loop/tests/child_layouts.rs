//! The child modules a render reports, found in the bar tree and in its open floating content,
//! with a failed child laid out as a placeholder.
use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::module_runtime::domain::{RenderOutcome, RenderPipeline};
use crate::features::module_runtime::test_support::{
    PipelineFixture, TestModulePort, process_monitor,
};
use crate::features::vdom::domain::{
    ElementSelectors, Interactions, ModuleParams, PopupSpec, VNode,
};
use crate::shared::primitives::{LayoutSurface, ModuleKey, ModuleName, ModuleOptions, MonitorId};
use std::collections::HashMap;

fn rect() -> VNode {
    VNode::new_rect(ElementSelectors::default(), Interactions::default())
}

fn module(name: &str) -> VNode {
    VNode::new_module(
        ModuleParams::new(ModuleName::new(name), None, ModuleOptions::default()),
        ElementSelectors::default(),
        Interactions::default(),
    )
}

fn flex(children: Vec<VNode>) -> VNode {
    VNode::new_flex(
        children,
        ElementSelectors::default(),
        Interactions::default(),
    )
}

/// The name and surface of each child layout `outcome` reports, in order.
fn children(outcome: &RenderOutcome) -> Vec<(&str, LayoutSurface)> {
    outcome
        .child_layouts()
        .iter()
        .map(|layout| (layout.key().name().as_str(), layout.surface()))
        .collect()
}

/// Modules embedded in the bar come first, then those in its open popup.
#[test]
fn child_layouts_list_the_bar_modules_then_the_floating_ones() {
    let mut pipeline = RenderPipeline::new();
    let mut fixture = PipelineFixture::<TaffyLayoutAdapter>::default();
    let bar = flex(vec![module("clock"), rect()])
        .with_popup(PopupSpec::new(Box::new(module("calendar"))));

    let outcome = process_monitor(
        &mut pipeline,
        &TestModulePort::new(bar),
        fixture.ctx(&MonitorId::new("DP-1")),
    )
    .expect("render outcome");

    assert_eq!(
        children(&outcome),
        vec![
            ("clock", LayoutSurface::Bar),
            ("calendar", LayoutSurface::Popup)
        ]
    );
}

/// A child whose lazy spawn failed is laid out as an error placeholder, so it is no longer
/// reported as an embedded module; its siblings still are.
#[test]
fn a_failed_child_is_laid_out_as_a_placeholder_instead_of_a_module() {
    let port = TestModulePort::new(flex(vec![module("calendar"), module("clock")]));
    let mon = MonitorId::new("DP-1");
    let mut fixture = PipelineFixture::<TaffyLayoutAdapter>::default();
    let healthy = process_monitor(&mut RenderPipeline::new(), &port, fixture.ctx(&mon))
        .expect("render outcome");
    let names: Vec<_> = children(&healthy)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(names, ["calendar", "clock"]);

    let errors = HashMap::from([(
        ModuleKey::from_name(ModuleName::new("calendar")),
        "not found".to_string(),
    )]);
    let mut ctx = fixture.ctx(&mon);
    ctx.child_errors = &errors;
    let failed = process_monitor(&mut RenderPipeline::new(), &port, ctx).expect("render outcome");
    let names: Vec<_> = children(&failed)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(names, ["clock"]);
}
