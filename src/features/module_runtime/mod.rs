//! Runs each loaded script module as its own actor.
//!
//! Holds the port every scripting engine implements, the per-module event loop, and the render
//! pipeline that turns a module's `VNode` tree into a painted buffer per monitor.

pub mod adapters;
pub mod application;
pub mod domain;
pub mod ports;

/// Test doubles for the module runtime's ports (senders, surface manager, canvas, layout engine,
/// style resolver, module port), shared by unit tests across the crate.
#[cfg(test)]
pub mod test_support {
    use crate::features::layout_engine::domain::{
        DisplayCommand, DisplayCommandSender, TextMeasurer,
    };
    use crate::features::module_runtime::ports::{
        AnyModulePort, LayoutEvent, LayoutEventSender, ModuleInitError,
    };
    use crate::features::vdom::domain::{UiCommand, UiCommandSender, VNode};
    use crate::shared::config::domain::{Config, ModuleConfig};
    use crate::shared::events::signals::{SignalHub, SignalKind};
    use crate::shared::primitives::geometry::{Position, Rect, Scale, Size};
    use crate::shared::primitives::render::ModuleFrame;
    use crate::shared::primitives::{FunctionName, MonitorId};
    use crate::shared::rendering::ports::canvas::{Canvas, CanvasFactory};
    use crate::shared::rendering::ports::draw::{
        BorderStroke, PaintTarget, RgbaImage, RoundedRect,
    };
    use crate::shared::rendering::ports::font::Font;
    use crate::shared::rendering::ports::text::{TextFont, TextPlacement, TextRun};

    pub(crate) struct MockLayoutSender;
    impl LayoutEventSender for MockLayoutSender {
        fn send_layout_event(&self, _event: LayoutEvent) {}
    }

    pub(crate) struct ChannelLayoutSender {
        pub(crate) tx: std::sync::mpsc::Sender<LayoutEvent>,
    }
    impl ChannelLayoutSender {
        #[must_use]
        pub(crate) const fn new(tx: std::sync::mpsc::Sender<LayoutEvent>) -> Self {
            Self { tx }
        }
    }
    impl LayoutEventSender for ChannelLayoutSender {
        fn send_layout_event(&self, event: LayoutEvent) {
            let _ = self.tx.send(event);
        }
    }

    pub(crate) struct MockDisplaySender;
    impl DisplayCommandSender for MockDisplaySender {
        fn send_display_command(&self, _cmd: DisplayCommand) {}
    }

    pub(crate) struct ChannelDisplaySender {
        pub(crate) tx: std::sync::mpsc::Sender<DisplayCommand>,
    }
    impl ChannelDisplaySender {
        #[must_use]
        pub(crate) const fn new(tx: std::sync::mpsc::Sender<DisplayCommand>) -> Self {
            Self { tx }
        }
    }
    impl DisplayCommandSender for ChannelDisplaySender {
        fn send_display_command(&self, cmd: DisplayCommand) {
            let _ = self.tx.send(cmd);
        }
    }

    pub(crate) struct MockUiSender;
    impl UiCommandSender for MockUiSender {
        fn send_ui_command(&self, _cmd: UiCommand) {}
    }

    pub(crate) struct ChannelUiSender {
        pub(crate) tx: std::sync::mpsc::Sender<UiCommand>,
    }
    impl UiCommandSender for ChannelUiSender {
        fn send_ui_command(&self, cmd: UiCommand) {
            let _ = self.tx.send(cmd);
        }
    }

    pub(crate) struct MockSurfaceManager;

    impl crate::shared::wayland::ports::SurfaceManagerPort for MockSurfaceManager {
        fn submit_buffer(&self, _frame: ModuleFrame) {}
    }

    /// Keeps every frame submitted to it, with the parent surface it was submitted under.
    #[derive(Default)]
    pub(crate) struct RecordingSurfaceManager {
        pub(crate) frames: std::sync::Mutex<
            Vec<(
                ModuleFrame,
                Option<crate::shared::wayland::ports::SurfaceParent>,
            )>,
        >,
    }

    impl crate::shared::wayland::ports::SurfaceManagerPort for RecordingSurfaceManager {
        fn submit_buffer(&self, frame: ModuleFrame) {
            self.frames.lock().unwrap().push((frame, None));
        }

        fn submit_child_buffer(
            &self,
            frame: ModuleFrame,
            parent: Option<crate::shared::wayland::ports::SurfaceParent>,
        ) {
            self.frames.lock().unwrap().push((frame, parent));
        }
    }

    #[derive(Debug, Default, Clone)]
    pub(crate) struct MockCanvasFactory;

    impl CanvasFactory for MockCanvasFactory {
        fn create_canvas(&mut self, _target: PaintTarget<'_>, _font: Font) -> impl Canvas + '_ {
            MockCanvas
        }

        fn create_text_measurer(&mut self, _scale: Scale, _font: Font) -> impl TextMeasurer + '_ {
            MockMeasurer
        }
    }

    pub(crate) struct MockCanvas;

    impl Canvas for MockCanvas {
        fn draw_rect(
            &mut self,
            _shape: RoundedRect,
            _color: crate::shared::primitives::color::DrawingColor,
        ) {
        }

        fn draw_border(&mut self, _shape: RoundedRect, _stroke: BorderStroke) {}

        fn draw_text(&mut self, _run: TextRun<'_>, _placement: TextPlacement) {}

        fn draw_image(&mut self, _image: RgbaImage<'_>, _area: Rect) {}
    }

    pub(crate) struct MockMeasurer;

    impl TextMeasurer for MockMeasurer {
        fn measure(&mut self, _text: &str, _font: TextFont<'_>) -> Size {
            Size::new(10, 10)
        }
    }

    pub(crate) struct TestModulePort {
        pub(crate) node: VNode,
        pub(crate) subs: Vec<SignalKind>,
    }

    impl TestModulePort {
        #[must_use]
        pub(crate) const fn new(node: VNode) -> Self {
            Self {
                node,
                subs: Vec::new(),
            }
        }

        #[must_use]
        pub(crate) const fn with_subs(node: VNode, subs: Vec<SignalKind>) -> Self {
            Self { node, subs }
        }
    }

    impl AnyModulePort for TestModulePort {
        fn init(
            &mut self,
            _config: &ModuleConfig,
            _full_config: &Config,
        ) -> Result<(), ModuleInitError> {
            Ok(())
        }

        fn subscriptions(&self) -> &[SignalKind] {
            &self.subs
        }

        fn styles(&self) -> &[crate::features::styling::domain::StyleSheetName] {
            &[]
        }

        fn refresh(&mut self, _hub: &SignalHub, _signals: &[SignalKind]) {}

        fn render(&self, _monitor: &MonitorId) -> VNode {
            self.node.clone()
        }

        fn call_function_with_args(
            &mut self,
            _name: &FunctionName,
            _args: &[&str],
        ) -> Result<(), ModuleInitError> {
            Ok(())
        }
    }

    #[derive(Debug, Default, Clone)]
    pub(crate) struct MockVdomDiff;

    impl crate::features::vdom::ports::VdomDiffPort for MockVdomDiff {
        fn diff<'a>(
            &self,
            old_tree: Option<&'a VNode>,
            new_tree: &'a VNode,
        ) -> crate::features::vdom::domain::DiffResult {
            if old_tree.is_some_and(|old| old == new_tree) {
                crate::features::vdom::domain::DiffResult::unchanged()
            } else {
                crate::features::vdom::domain::DiffResult::new(
                    crate::features::vdom::domain::Patch::Replace {
                        old_node_id: crate::features::vdom::domain::NodeId::new(),
                        new_node: Box::new(new_tree.clone()),
                    },
                )
            }
        }
    }

    #[derive(Debug, Default, Clone)]
    pub(crate) struct MockStyleResolver;

    impl crate::features::styling::ports::StyleResolverPort for MockStyleResolver {
        fn resolve_style(
            &self,
            _query: &crate::features::styling::domain::ElementQuery<'_>,
            _inherited: &crate::features::styling::domain::InheritedStyle,
        ) -> (
            crate::features::styling::domain::ComputedStyle,
            crate::features::styling::domain::InheritedStyle,
        ) {
            (
                crate::features::styling::domain::ComputedStyle::default(),
                crate::features::styling::domain::InheritedStyle::default(),
            )
        }
    }

    /// Owns everything a `LayoutContext` borrows, so a test builds one per render in one call:
    /// scale 1, no bounds, constraint, child sizes or interaction, which the test then sets.
    #[derive(Default)]
    pub(crate) struct PipelineFixture<E> {
        resolver: MockStyleResolver,
        factory: MockCanvasFactory,
        child_errors: std::collections::HashMap<crate::shared::primitives::ModuleKey, String>,
        engine: E,
        popup_engine: E,
        panel_engine: E,
        tooltip_engine: E,
    }

    impl<E: crate::features::layout_engine::ports::LayoutEnginePort> PipelineFixture<E> {
        pub(crate) fn ctx<'a>(
            &'a mut self,
            monitor_id: &'a MonitorId,
        ) -> crate::features::module_runtime::domain::LayoutContext<'a, MockCanvasFactory, E>
        {
            crate::features::module_runtime::domain::LayoutContext {
                monitor_id,
                scale: Scale::new(1.0),
                style_resolver: &self.resolver,
                child_errors: &self.child_errors,
                current_bounds: None,
                current_constraint: crate::shared::primitives::SizeConstraint::none(),
                current_child_sizes: None,
                interaction_context: None,
                canvas_factory: &mut self.factory,
                layout_engine: &mut self.engine,
                popup_layout_engine: &mut self.popup_engine,
                panel_layout_engine: &mut self.panel_engine,
                tooltip_layout_engine: &mut self.tooltip_engine,
            }
        }
    }

    /// Runs every phase of one render on the context's monitor, as the event loop does, comparing
    /// trees with `MockVdomDiff`.
    pub(crate) fn process_monitor<F: CanvasFactory, E>(
        pipeline: &mut crate::features::module_runtime::domain::RenderPipeline,
        port: &dyn AnyModulePort,
        ctx: crate::features::module_runtime::domain::LayoutContext<'_, F, E>,
    ) -> Option<crate::features::module_runtime::domain::RenderOutcome>
    where
        E: crate::features::layout_engine::ports::LayoutEnginePort,
    {
        let diff = pipeline.diff(port, ctx.diff_input(&MockVdomDiff))?;
        pipeline.layout_and_paint(diff, ctx)
    }

    #[derive(Debug, Default, Clone)]
    pub(crate) struct MockLayoutEngine;

    impl crate::features::layout_engine::ports::LayoutEnginePort for MockLayoutEngine {
        fn calculate_layout_with_constraints(
            &mut self,
            _node: crate::features::layout_engine::domain::StyledNode,
            _measurer: &mut dyn TextMeasurer,
            _start_pos: Position,
            _constraint: crate::shared::primitives::SizeConstraint,
        ) -> Result<
            crate::features::layout_engine::domain::RenderNode,
            crate::features::layout_engine::domain::LayoutError,
        > {
            Ok(crate::features::layout_engine::domain::RenderNode::Rect {
                path: crate::features::layout_engine::domain::NodePath::root_in(
                    crate::features::layout_engine::domain::SurfaceSpace::Bar,
                ),
                node_key: None,
                rect: crate::shared::primitives::geometry::Rect::new(
                    Position::new(0, 0),
                    Size::new(10, 10),
                ),
                style: crate::features::styling::domain::ComputedStyle::default(),
                on_click: None,
                on_hover: None,
                tooltip: None,
                popup: None,
                panel: None,
            })
        }
    }
}
