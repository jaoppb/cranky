pub mod adapters;
pub mod application;
pub mod domain;
pub mod ports;

#[cfg(test)]
pub mod test_support {
    use crate::features::layout_engine::domain::{
        DisplayCommand, DisplayCommandSender, TextMeasurer,
    };
    use crate::features::module_runtime::ports::{
        AnyModulePort, LayoutEvent, LayoutEventSender, ModuleInitError,
    };
    use crate::features::vdom::domain::{UiCommand, UiCommandSender, VNode};
    use crate::shared::config::domain::{Config, FontFamily, FontSize, ModuleConfig};
    use crate::shared::events::signals::{SignalHub, SignalKind};
    use crate::shared::primitives::geometry::{Position, Scale, Size};
    use crate::shared::primitives::render::RenderBuffer;
    use crate::shared::primitives::{FunctionName, ModuleId, MonitorId};
    use crate::shared::rendering::ports::canvas::{Canvas, CanvasFactory};

    pub struct MockLayoutSender;
    impl LayoutEventSender for MockLayoutSender {
        fn send_layout_event(&self, _event: LayoutEvent) {}
    }

    pub struct ChannelLayoutSender {
        pub tx: std::sync::mpsc::Sender<LayoutEvent>,
    }
    impl ChannelLayoutSender {
        #[must_use]
        pub const fn new(tx: std::sync::mpsc::Sender<LayoutEvent>) -> Self {
            Self { tx }
        }
    }
    impl LayoutEventSender for ChannelLayoutSender {
        fn send_layout_event(&self, event: LayoutEvent) {
            let _ = self.tx.send(event);
        }
    }

    pub struct MockDisplaySender;
    impl DisplayCommandSender for MockDisplaySender {
        fn send_display_command(&self, _cmd: DisplayCommand) {}
    }

    pub struct ChannelDisplaySender {
        pub tx: std::sync::mpsc::Sender<DisplayCommand>,
    }
    impl ChannelDisplaySender {
        #[must_use]
        pub const fn new(tx: std::sync::mpsc::Sender<DisplayCommand>) -> Self {
            Self { tx }
        }
    }
    impl DisplayCommandSender for ChannelDisplaySender {
        fn send_display_command(&self, cmd: DisplayCommand) {
            let _ = self.tx.send(cmd);
        }
    }

    pub struct MockUiSender;
    impl UiCommandSender for MockUiSender {
        fn send_ui_command(&self, _cmd: UiCommand) {}
    }

    pub struct ChannelUiSender {
        pub tx: std::sync::mpsc::Sender<UiCommand>,
    }
    impl ChannelUiSender {
        #[must_use]
        pub const fn new(tx: std::sync::mpsc::Sender<UiCommand>) -> Self {
            Self { tx }
        }
    }
    impl UiCommandSender for ChannelUiSender {
        fn send_ui_command(&self, cmd: UiCommand) {
            let _ = self.tx.send(cmd);
        }
    }

    pub struct MockSurfaceManager;

    impl crate::shared::wayland::ports::SurfaceManagerPort for MockSurfaceManager {
        fn submit_buffer(
            &self,
            _mod_id: ModuleId,
            _mon_id: MonitorId,
            _pos: Position,
            _buf: RenderBuffer,
        ) {
        }
    }

    #[derive(Debug, Default, Clone)]
    pub struct MockCanvasFactory;

    impl CanvasFactory for MockCanvasFactory {
        fn create_canvas(
            &mut self,
            _data: &mut [u8],
            _size: Size,
            _scale: Scale,
            _font_family: FontFamily,
            _font_size: FontSize,
        ) -> impl Canvas + '_ {
            MockCanvas
        }

        fn create_text_measurer(
            &mut self,
            _scale: Scale,
            _font_family: FontFamily,
            _font_size: FontSize,
        ) -> impl TextMeasurer + '_ {
            MockMeasurer
        }
    }

    pub struct MockCanvas;

    impl Canvas for MockCanvas {
        fn draw_rect(
            &mut self,
            _x: crate::shared::primitives::geometry::LogicalPx,
            _y: crate::shared::primitives::geometry::LogicalPx,
            _w: crate::shared::primitives::geometry::LogicalPx,
            _h: crate::shared::primitives::geometry::LogicalPx,
            _color: crate::shared::primitives::color::DrawingColor,
            _radius: crate::shared::primitives::geometry::LogicalPx,
        ) {
        }

        fn draw_border(
            &mut self,
            _pos: Position,
            _size: Size,
            _color: crate::shared::primitives::color::DrawingColor,
            _radius: crate::shared::primitives::geometry::LogicalPx,
            _border_size: crate::shared::primitives::geometry::LogicalPx,
        ) {
        }

        fn draw_text(
            &mut self,
            _text: &str,
            _font_family: Option<&FontFamily>,
            _font_size: Option<FontSize>,
            _color: crate::shared::primitives::color::DrawingColor,
            _pos: Position,
        ) {
        }

        fn draw_image(
            &mut self,
            _image_data: &[u8],
            _pixel_size: Size,
            _logical_size: Size,
            _pos: Position,
        ) {
        }
    }

    pub struct MockMeasurer;

    impl TextMeasurer for MockMeasurer {
        fn measure(
            &mut self,
            _text: &str,
            _font_family: Option<&FontFamily>,
            _font_size: Option<FontSize>,
        ) -> Size {
            Size::new(10, 10)
        }
    }

    pub struct TestModulePort {
        pub node: VNode,
        pub subs: Vec<SignalKind>,
    }

    impl TestModulePort {
        #[must_use]
        pub const fn new(node: VNode) -> Self {
            Self {
                node,
                subs: Vec::new(),
            }
        }

        #[must_use]
        pub const fn with_subs(node: VNode, subs: Vec<SignalKind>) -> Self {
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

        fn call_function(&mut self, _name: &FunctionName) -> Result<(), ModuleInitError> {
            Ok(())
        }
    }

    #[derive(Debug, Default, Clone)]
    pub struct MockVdomDiff;

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
    pub struct MockStyleResolver;

    impl crate::features::styling::ports::StyleResolverPort for MockStyleResolver {
        fn resolve_style(
            &self,
            _query: &crate::features::styling::domain::ElementQuery,
        ) -> crate::features::styling::domain::ComputedStyle {
            crate::features::styling::domain::ComputedStyle::default()
        }
    }

    #[derive(Debug, Default, Clone)]
    pub struct MockLayoutEngine;

    impl crate::features::layout_engine::ports::LayoutEnginePort for MockLayoutEngine {
        fn calculate_layout_with_constraints(
            &mut self,
            _node: crate::features::layout_engine::domain::StyledNode,
            _measurer: &mut dyn TextMeasurer,
            _start_pos: Position,
            _available_size: Option<Size>,
        ) -> Result<
            crate::features::layout_engine::domain::RenderNode,
            crate::features::layout_engine::domain::LayoutError,
        > {
            Ok(crate::features::layout_engine::domain::RenderNode::Rect {
                path: crate::features::layout_engine::domain::NodePath::root(),
                rect: crate::shared::primitives::geometry::Rect::new(
                    Position::new(0, 0),
                    Size::new(10, 10),
                ),
                style: crate::features::styling::domain::ComputedStyle::default(),
                on_click: None,
                on_hover: None,
                tooltip: None,
            })
        }
    }
}
