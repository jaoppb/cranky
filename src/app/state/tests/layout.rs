//! How the app records where parents lay their children out.

use super::fixtures::{app_over, key, laid_out, module_on};
use crate::app::test_fakes::FakeRegistry;
use crate::features::module_runtime::test_support::capture_logs;
use crate::shared::config::domain::Config;
use crate::shared::primitives::{LayoutSurface, ModuleId, MonitorId};

/// A registry where module 1 embeds module 2 at every key.
fn parent_embedding_child() -> FakeRegistry {
    let mut registry = FakeRegistry::default();
    registry.resolve_site =
        Box::new(|parent, _| (parent == Some(ModuleId::new(1))).then(|| ModuleId::new(2)));
    registry.parent_of = Box::new(|id| (id == ModuleId::new(2)).then(|| ModuleId::new(1)));
    registry
}

/// Placing a child and withdrawing its site are each traced with the child and the monitor.
#[test]
fn test_handle_container_layouts_traces_each_placed_and_withdrawn_child() {
    let logs = capture_logs(|| {
        crate::test_utils::block_on(async {
            let (mut app, _hub) = app_over(Config::default(), parent_embedding_child());
            let monitor = MonitorId::new("DP-1");
            let child = laid_out(key("calendar"), 30, LayoutSurface::Popup);

            app.handle_container_layouts(module_on(ModuleId::new(1), &monitor), &[child])
                .await;
            app.handle_container_layouts(module_on(ModuleId::new(1), &monitor), &[])
                .await;
        });
    });

    assert_eq!(
        logs.lines().collect::<Vec<_>>(),
        [
            "TRACE cranky::app::state: Updating computed_layouts for child module child=2 monitor=DP-1 bounds=Rect { position: Position { x: 30, y: 0 }, size: Size { width: 50, height: 20 } }",
            "TRACE cranky::app::state: Embedding site withdrawn; suspending child on this monitor child=2 monitor=DP-1",
        ]
    );
}
